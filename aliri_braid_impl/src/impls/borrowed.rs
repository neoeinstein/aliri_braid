use crate::check_mode::CheckMode;
use crate::codegen::Parameters;
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
        check_mode,
        omit_debug,
        omit_display,
        derive_serde,
    } = params;

    let (_wrapped_type, field_ident) = get_or_set_wrapped_ref_type(&mut body.fields)?;
    let field = field_ident
        .as_ref()
        .map_or_else(|| quote!{0}, |i| i.to_token_stream());
    let inherent_impl = inherent_impl(&body.ident, &owned_type, &check_mode, &field);

    let comparison_impls = owned_type
        .as_ref()
        .map(|owned_type| comparison_impls(&body.ident, owned_type, &field_ident));
    let conversion_impls = conversion_impls(&body.ident, &check_mode, &field);
    let display_impl = (!omit_display).then(|| display_impl(&body.ident, &field));
    let debug_impl = (!omit_debug).then(|| debug_impl(&body.ident, &field));
    let serde_impls = derive_serde.then(|| serde_impls(&body.ident, &owned_type, &check_mode));

    let output = quote! {
        #[repr(transparent)]
        #[derive(Hash, PartialEq, Eq, PartialOrd, Ord)]
        #body

        #inherent_impl
        #comparison_impls
        #conversion_impls
        #debug_impl
        #display_impl
        #serde_impls
    };

    Ok(output)
}

fn get_or_set_wrapped_ref_type(
    fields: &mut syn::Fields,
) -> Result<(syn::Type, Option<syn::Ident>), syn::Error> {
    if fields.is_empty() {
        let def_type: syn::Type = syn::parse2(quote! { str }).unwrap();
        let flds = syn::parse2(quote! { (#def_type) }).unwrap();
        *fields = syn::Fields::Unnamed(flds);
        Ok((def_type, None))
    } else if let syn::Fields::Unnamed(flds) = &mut *fields {
        let mut iter = flds.unnamed.iter();
        let f = iter.next().unwrap();
        if iter.next().is_some() {
            Err(syn::Error::new_spanned(
                &flds,
                "typed string can only have one field",
            ))
        } else {
            Ok((f.ty.clone(), f.ident.clone()))
        }
    } else if let syn::Fields::Named(flds) = &mut *fields {
        let mut iter = flds.named.iter();
        let f = iter.next().unwrap();
        if iter.next().is_some() {
            Err(syn::Error::new_spanned(
                &flds,
                "typed string can only have one field",
            ))
        } else {
            Ok((f.ty.clone(), f.ident.clone()))
        }
    } else {
        Err(syn::Error::new_spanned(
            &fields,
            "typed string can only have one field",
        ))
    }
}

fn conversion_impls(
    name: &syn::Ident,
    check_mode: &CheckMode,
    field: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {

}

fn display_impl(name: &syn::Ident, field: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    quote! {
        impl ::std::fmt::Display for #name {
            #[inline]
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                <str as ::std::fmt::Display>::fmt(&self.#field, f)
            }
        }
    }
}

fn debug_impl(name: &syn::Ident, field: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    quote! {
        impl ::std::fmt::Debug for #name {
            #[inline]
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                <str as ::std::fmt::Debug>::fmt(&self.#field, f)
            }
        }
    }
}

fn fallible_serde_tokens() -> proc_macro2::TokenStream {
    quote! {.map_err(<D::Error as ::serde::de::Error>::custom)?}
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use quote::format_ident;
    use syn::parse_quote;

    fn borrowed_ident() -> syn::Ident {
        format_ident!("Borrowed")
    }

    fn owned_type() -> syn::Type {
        parse_quote! { Owned }
    }

    fn validating_type() -> syn::Type {
        parse_quote! { TheValidator }
    }

    #[test]
    fn expected_serde_impls_owned_infallible() {
        let name = borrowed_ident();
        let owned: syn::Type = owned_type();

        let actual = serde_impls(&name, &Some(owned), &CheckMode::None);
        let expected = quote! {
            impl ::serde::Serialize for Borrowed {
                fn serialize<S: ::serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                    <str as ::serde::Serialize>::serialize(self.as_str(), serializer)
                }
            }

            #[allow(clippy::needless_question_mark)]
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

        let actual = serde_impls(&name, &Some(owned), &CheckMode::Validate(validating_type()));
        let expected = quote! {
            impl ::serde::Serialize for Borrowed {
                fn serialize<S: ::serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                    <str as ::serde::Serialize>::serialize(self.as_str(), serializer)
                }
            }

            #[allow(clippy::needless_question_mark)]
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

        let actual = serde_impls(&name, &None, &CheckMode::None);
        let expected = quote! {
            impl ::serde::Serialize for Borrowed {
                fn serialize<S: ::serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                    <str as ::serde::Serialize>::serialize(self.as_str(), serializer)
                }
            }

            #[allow(clippy::needless_question_mark)]
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

        let actual = serde_impls(&name, &None, &CheckMode::Validate(validating_type()));
        let expected = quote! {
            impl ::serde::Serialize for Borrowed {
                fn serialize<S: ::serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                    <str as ::serde::Serialize>::serialize(self.as_str(), serializer)
                }
            }

            #[allow(clippy::needless_question_mark)]
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
