mod parameters;

pub use parameters::Parameters;
use quote::{quote, ToTokens};
use std::convert::TryInto;

pub fn typed_string_ref_tokens(
    args: syn::AttributeArgs,
    body: syn::ItemStruct,
) -> Result<proc_macro2::TokenStream, syn::Error> {
    typed_string_ref_params(args.try_into()?, body)
}

pub fn typed_string_ref_params(
    params: Parameters,
    mut body: syn::ItemStruct,
) -> Result<proc_macro2::TokenStream, syn::Error> {
    let Parameters {
        owned_type,
        validator,
        derive_serde,
    } = params;

    let _wrapped_type = get_or_set_wrapped_ref_type(&mut body.fields)?;
    let inherent_impl = inherent_impl(&body.ident, &owned_type, &validator);

    let comparison_impls = owned_type
        .as_ref()
        .map(|owned_type| comparison_impls(&body.ident, owned_type));
    let conversion_impls = conversion_impls(&body.ident, &validator);
    let common_impls = common_impls(&body.ident);
    let serde_impls =
        derive_serde.then(|| serde_impls(&body.ident, &owned_type, validator.is_some()));

    let output = quote! {
        #[derive(Debug, Hash, PartialEq, Eq)]
        #[repr(transparent)]
        #body

        #inherent_impl
        #comparison_impls
        #conversion_impls
        #common_impls
        #serde_impls
    };

    Ok(output)
}

fn get_or_set_wrapped_ref_type(fields: &mut syn::Fields) -> Result<syn::Type, syn::Error> {
    if fields.is_empty() {
        let def_type: syn::Type = syn::parse2(quote! { str }).unwrap();
        let flds = syn::parse2(quote! { (#def_type) }).unwrap();
        *fields = syn::Fields::Unnamed(flds);
        Ok(def_type)
    } else if let syn::Fields::Unnamed(flds) = &mut *fields {
        let mut iter = flds.unnamed.iter();
        let f = iter.next().unwrap();
        if iter.next().is_some() {
            Err(syn::Error::new_spanned(
                &flds,
                "typed string can only have one unnamed field",
            ))
        } else {
            Ok(f.ty.clone())
        }
    } else {
        Err(syn::Error::new_spanned(
            &fields,
            "typed string can only have one unnamed field",
        ))
    }
}

fn inherent_impl(
    name: &syn::Ident,
    owned_type: &Option<syn::Type>,
    validator: &Option<syn::Type>,
) -> proc_macro2::TokenStream {
    let creation_functions = if let Some(v) = validator {
        fallible_ref_creation(name, owned_type, v)
    } else {
        infallible_ref_creation(name, owned_type)
    };

    quote! {
        impl #name {
            #creation_functions

            /// Provides access to the underlying value as a string slice.
            #[inline]
            pub const fn as_str(&self) -> &str {
                &self.0
            }
        }
    }
}

fn infallible_ref_creation(
    name: &syn::Ident,
    owned_type: &Option<syn::Type>,
) -> proc_macro2::TokenStream {
    let doc_comment = format!(
        "Transparently reinterprets the string slice as a strongly-typed {}",
        name
    );

    let box_into_owned = owned_type.as_ref().map(|owned_type| {
        let into_owned_doc = format!(
            "Converts a [`Box<{}>`] into a [`{}`] without copying or allocating",
            name,
            owned_type.to_token_stream(),
        );

        quote! {
            #[inline]
            #[doc = #into_owned_doc]
            pub fn into_owned(self: Box<#name>) -> #owned_type {
                // Safety: The representation of `Self` should be exactly the same
                // as a `Box<str>`.
                let s: Box<str> = unsafe { ::std::mem::transmute(self) };
                #owned_type::new(s.into_string())
            }
        }
    });

    let creation_functions = quote! {
        #[allow(unsafe_code)]
        #[inline]
        #[doc = #doc_comment]
        pub fn from_str(raw: &str) -> &Self {
            let ptr: *const str = raw;
            // This type is a transparent wrapper around an `str` slice, so this
            // transformation is safe to do.
            unsafe {
                &*(ptr as *const Self)
            }
        }

        #box_into_owned
    };

    creation_functions
}

fn fallible_ref_creation(
    name: &syn::Ident,
    owned_type: &Option<syn::Type>,
    validator: &syn::Type,
) -> proc_macro2::TokenStream {
    let doc_comment = format!(
        "Transparently reinterprets the string slice as a strongly-typed {} \
        if it conforms to [`{}`]",
        name,
        validator.to_token_stream(),
    );

    let doc_comment_unsafe = format!(
        "Transparently reinterprets the string slice as a strongly-typed {} \
        without validating",
        name,
    );

    let box_into_owned = owned_type.as_ref().map(|owned_type| {
        let into_owned_doc = format!(
            "Converts a [`Box<{}>`] into a [`{}`] without copying or allocating",
            name,
            owned_type.to_token_stream(),
        );

        quote! {
            #[allow(unsafe_code)]
            #[inline]
            #[doc = #into_owned_doc]
            pub fn into_owned(self: Box<#name>) -> #owned_type {
                // Safety: The representation of `Self` should be exactly the same
                // as a `Box<str>`.
                let s: Box<str> = unsafe { ::std::mem::transmute(self) };
                let s = s.into_string();
                // Safety: As a precondition of being this type, any validator will have
                // already validated that `s` is valid.
                unsafe { #owned_type::new_unchecked(s) }
            }
        }
    });

    let creation_functions = quote! {
        #[allow(unsafe_code)]
        #[inline]
        #[doc = #doc_comment]
        pub fn from_str(raw: &str) -> Result<&Self, <#validator as ::braid::Validator>::Error> {
            <#validator as ::braid::Validator>::validate(raw)?;

            let ptr: *const str = raw;
            // This type is a transparent wrapper around an `str` slice, so this
            // transformation is safe to do.
            unsafe {
                Ok(&*(ptr as *const Self))
            }
        }

        #[allow(unsafe_code)]
        #[inline]
        #[doc = #doc_comment_unsafe]
        pub unsafe fn from_str_unchecked(raw: &str) -> &Self {
            let ptr: *const str = raw;
            // This type is a transparent wrapper around an `str` slice, so this
            // transformation is safe to do.
            unsafe {
                &*(ptr as *const Self)
            }
        }

        #box_into_owned
    };

    creation_functions
}

fn comparison_impls(name: &syn::Ident, owned_type: &syn::Type) -> proc_macro2::TokenStream {
    quote! {
        impl ToOwned for #name {
            type Owned = #owned_type;

            #[inline]
            fn to_owned(&self) -> Self::Owned {
                #owned_type(self.0.to_owned())
            }
        }

        impl PartialEq<#name> for #owned_type {
            #[inline]
            fn eq(&self, other: &#name) -> bool {
                self.as_str() == other.as_str()
            }
        }

        impl PartialEq<&'_ #name> for #owned_type {
            #[inline]
            fn eq(&self, other: &&#name) -> bool {
                self.as_str() == other.as_str()
            }
        }

        impl PartialEq<Box<#name>> for #owned_type {
            #[inline]
            fn eq(&self, other: &Box<#name>) -> bool {
                self.as_str() == other.as_str()
            }
        }

        impl PartialEq<Box<#name>> for &'_ #name {
            #[inline]
            fn eq(&self, other: &Box<#name>) -> bool {
                self.as_str() == other.as_str()
            }
        }

        impl PartialEq<#owned_type> for #name {
            #[inline]
            fn eq(&self, other: &#owned_type) -> bool {
                self.as_str() == other.as_str()
            }
        }

        impl PartialEq<#owned_type> for &'_ #name {
            #[inline]
            fn eq(&self, other: &#owned_type) -> bool {
                self.as_str() == other.as_str()
            }
        }

        impl PartialEq<#owned_type> for Box<#name> {
            #[inline]
            fn eq(&self, other: &#owned_type) -> bool {
                self.as_str() == other.as_str()
            }
        }

        impl PartialEq<&'_ #name> for Box<#name> {
            #[inline]
            fn eq(&self, other: &&#name) -> bool {
                self.as_str() == other.as_str()
            }
        }
    }
}

fn conversion_impls(name: &syn::Ident, validator: &Option<syn::Type>) -> proc_macro2::TokenStream {
    if let Some(validator) = validator {
        quote! {
            impl<'a> std::convert::TryFrom<&'a str> for &'a #name {
                type Error = <#validator as ::braid::Validator>::Error;
                fn try_from(s: &'a str) -> Result<&'a #name, Self::Error> {
                    #name::from_str(s)
                }
            }
        }
    } else {
        quote! {
            impl<'a> From<&'a str> for &'a #name {
                fn from(s: &'a str) -> &'a #name {
                    #name::from_str(s)
                }
            }
        }
    }
}

fn common_impls(name: &syn::Ident) -> proc_macro2::TokenStream {
    quote! {
        impl AsRef<str> for #name {
            #[inline]
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl<'a> ::std::fmt::Display for &'a #name {
            #[inline]
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                f.write_str(&self.0)
            }
        }
    }
}

fn fallible_serde_tokens() -> proc_macro2::TokenStream {
    quote! {.map_err(<D::Error as ::serde::de::Error>::custom)?}
}

pub fn serde_impls(
    name: &syn::Ident,
    owned_type: &Option<syn::Type>,
    is_fallible: bool,
) -> proc_macro2::TokenStream {
    let handle_failure = is_fallible.then(fallible_serde_tokens);

    let boxed_impl = owned_type.as_ref().map(|owned_type| {
        quote! {
            impl<'de> ::serde::Deserialize<'de> for Box<#name> {
                fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                    let owned = <#owned_type as ::serde::Deserialize<'de>>::deserialize(deserializer)?;
                    Ok(owned.into_boxed_ref())
                }
            }
        }
    });

    quote! {
        impl ::serde::Serialize for #name {
            fn serialize<S: ::serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                <str as ::serde::Serialize>::serialize(self.as_str(), serializer)
            }
        }

        impl<'de: 'a, 'a> ::serde::Deserialize<'de> for &'a #name {
            fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let raw = <&str as ::serde::Deserialize<'de>>::deserialize(deserializer)?;
                Ok(#name::from_str(raw)#handle_failure)
            }
        }

        #boxed_impl
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::format_ident;
    use syn::parse_quote;

    fn borrowed_ident() -> syn::Ident {
        format_ident!("Borrowed")
    }

    fn owned_type() -> syn::Type {
        parse_quote! { Owned }
    }

    #[test]
    fn expected_serde_impls_owned_infallible() {
        let name = borrowed_ident();
        let owned: syn::Type = owned_type();

        let actual = serde_impls(&name, &Some(owned), false);
        let expected = quote! {
            impl ::serde::Serialize for Borrowed {
                fn serialize<S: ::serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                    <str as ::serde::Serialize>::serialize(self.as_str(), serializer)
                }
            }

            impl<'de: 'a, 'a> ::serde::Deserialize<'de> for &'a Borrowed {
                fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                    let raw = <&str as ::serde::Deserialize<'de>>::deserialize(deserializer)?;
                    Ok(Borrowed::from_str(raw))
                }
            }

            impl<'de> ::serde::Deserialize<'de> for Box<Borrowed> {
                fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                    let owned = <Owned as ::serde::Deserialize<'de>>::deserialize(deserializer)?;
                    Ok(owned.into_boxed_ref())
                }
            }
        };

        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn expected_serde_impls_owned_fallible() {
        let name = borrowed_ident();
        let owned: syn::Type = owned_type();

        let actual = serde_impls(&name, &Some(owned), true);
        let expected = quote! {
            impl ::serde::Serialize for Borrowed {
                fn serialize<S: ::serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                    <str as ::serde::Serialize>::serialize(self.as_str(), serializer)
                }
            }

            impl<'de: 'a, 'a> ::serde::Deserialize<'de> for &'a Borrowed {
                fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                    let raw = <&str as ::serde::Deserialize<'de>>::deserialize(deserializer)?;
                    Ok(Borrowed::from_str(raw).map_err(<D::Error as ::serde::de::Error>::custom)?)
                }
            }

            impl<'de> ::serde::Deserialize<'de> for Box<Borrowed> {
                fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                    let owned = <Owned as ::serde::Deserialize<'de>>::deserialize(deserializer)?;
                    Ok(owned.into_boxed_ref())
                }
            }
        };

        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn expected_serde_impls_no_owned_infallible() {
        let name = borrowed_ident();

        let actual = serde_impls(&name, &None, false);
        let expected = quote! {
            impl ::serde::Serialize for Borrowed {
                fn serialize<S: ::serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                    <str as ::serde::Serialize>::serialize(self.as_str(), serializer)
                }
            }

            impl<'de: 'a, 'a> ::serde::Deserialize<'de> for &'a Borrowed {
                fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                    let raw = <&str as ::serde::Deserialize<'de>>::deserialize(deserializer)?;
                    Ok(Borrowed::from_str(raw))
                }
            }
        };

        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn expected_serde_impls_no_owned_fallible() {
        let name = borrowed_ident();

        let actual = serde_impls(&name, &None, true);
        let expected = quote! {
            impl ::serde::Serialize for Borrowed {
                fn serialize<S: ::serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                    <str as ::serde::Serialize>::serialize(self.as_str(), serializer)
                }
            }

            impl<'de: 'a, 'a> ::serde::Deserialize<'de> for &'a Borrowed {
                fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                    let raw = <&str as ::serde::Deserialize<'de>>::deserialize(deserializer)?;
                    Ok(Borrowed::from_str(raw).map_err(<D::Error as ::serde::de::Error>::custom)?)
                }
            }
        };

        assert_eq!(expected.to_string(), actual.to_string());
    }
}
