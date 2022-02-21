mod parameters;

use crate::check_mode::CheckMode;
pub use parameters::Parameters;
use quote::{format_ident, quote, TokenStreamExt, ToTokens};
use std::convert::TryInto;

pub fn typed_string_tokens(
    args: syn::AttributeArgs,
    body: syn::ItemStruct,
) -> Result<proc_macro2::TokenStream, syn::Error> {
    typed_string_params(args.try_into()?, body)
}

pub fn typed_string_params(
    params: Parameters,
    mut body: syn::ItemStruct,
) -> Result<proc_macro2::TokenStream, syn::Error> {
    let Parameters {
        ref_type,
        omit_clone,
        impl_debug,
        impl_display,
        derive_serde,
        no_auto_ref,
        ref_doc,
        check_mode,
    } = params;
    let ref_type = ref_type.unwrap_or_else(|| infer_ref_type_from_owned_name(&body.ident));

    let (wrapped_type, field_ident, attrs) = get_or_set_wrapped_type(&mut body.fields)?;
    let name = &body.ident;
    let field = field_ident.as_ref().map_or_else(|| quote!{0}, |i| i.to_token_stream());
    let check_mode = check_mode.infer_validator_if_missing(name);

    let inherent_impls = inherent_impls(name, &ref_type, &wrapped_type, &check_mode, &field_ident, &field);
    let common_impls = common_impls(name, &ref_type, &field);
    let conversion_impls = conversion_impls(name, &ref_type, &wrapped_type, &check_mode, &field);

    let serde_impls = derive_serde.then(|| serde_impls(name, &check_mode, &wrapped_type, &field));

    let construct_ref_item = (!no_auto_ref)
        .then(|| {
            construct_ref_item(
                name,
                &body.vis,
                &ref_type,
                &field_ident,
                &attrs,
                crate::borrow::Parameters {
                    owned_type: Some(syn::parse_quote!(#name)),
                    check_mode,
                    omit_debug: impl_debug != parameters::AutoImplOption::Auto,
                    omit_display: impl_display != parameters::AutoImplOption::Auto,
                    derive_serde,
                },
                ref_doc,
            )
        })
        .transpose()?;

    let clone = (!omit_clone).then(|| quote! { #[derive(Clone)] });
    let debug_impl =
        (impl_debug != parameters::AutoImplOption::None).then(|| debug_impl(name, &ref_type));
    let display_impl =
        (impl_debug != parameters::AutoImplOption::None).then(|| display_impl(name, &ref_type));

    let output = quote! {
        #clone
        #[repr(transparent)]
        #body

        #inherent_impls
        #common_impls
        #debug_impl
        #display_impl
        #conversion_impls
        #serde_impls

        #construct_ref_item
    };

    Ok(output)
}

fn infer_ref_type_from_owned_name(name: &syn::Ident) -> syn::Type {
    let name_str = name.to_string();
    if name_str.ends_with("Buf") {
        syn::Type::Path(syn::TypePath {
            qself: None,
            path: syn::Path::from(format_ident!("{}", name_str[..name_str.len() - 3])),
        })
    } else {
        syn::Type::Path(syn::TypePath {
            qself: None,
            path: syn::Path::from(format_ident!("{}Ref", name_str)),
        })
    }
}

fn get_or_set_wrapped_type(fields: &mut syn::Fields) -> Result<(syn::Type, Option<syn::Ident>, Vec<syn::Attribute>), syn::Error> {
    if fields.is_empty() {
        let def_type: syn::Type = syn::parse2(quote! { String }).unwrap();
        let flds = syn::parse2(quote! { (#def_type) }).unwrap();
        *fields = syn::Fields::Unnamed(flds);
        Ok((def_type, None, Vec::new()))
    } else if let syn::Fields::Unnamed(flds) = &mut *fields {
        let mut iter = flds.unnamed.iter();
        let f = iter.next().unwrap();
        if iter.next().is_some() {
            Err(syn::Error::new_spanned(
                &flds,
                "typed string can only have one field",
            ))
        } else {
            Ok((f.ty.clone(), f.ident.clone(), f.attrs.clone()))
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
            Ok((f.ty.clone(), f.ident.clone(), f.attrs.clone()))
        }
    } else {
        Err(syn::Error::new_spanned(
            &fields,
            "typed string can only have one field",
        ))
    }
}

fn infallible_owned_creation(ident: &syn::Ident, field: &Option<syn::Ident>) -> proc_macro2::TokenStream {
    let doc_comment = format!("Constructs a new {}", ident);

    let create = if let Some(field) = field {
        quote! { Self { #field: s.into() } }
    } else {
        quote! { Self(s.into()) }
    };

    let creation_functions = quote! {
        #[doc = #doc_comment]
        #[inline]
        pub fn new<S: Into<String>>(s: S) -> Self {
            #create
        }
    };

    creation_functions
}

fn fallible_owned_creation(ident: &syn::Ident, field: &Option<syn::Ident>, validator: &syn::Type) -> proc_macro2::TokenStream {
    let validator_tokens = validator.to_token_stream();
    let doc_comment = format!(
        "Constructs a new {} if it conforms to [`{}`]",
        ident, validator_tokens
    );

    let doc_comment_unsafe = format!(
        "Constructs a new {} without validation\n\
        \n\
        ## Safety\n\
        \n\
        Consumers of this function must ensure that values conform to [`{}`]. \
        Failure to maintain this invariant may lead to undefined behavior.",
        ident, validator_tokens
    );

    let validator = super::as_validator(validator);

    let create = if let Some(field) = field {
        quote! { Self { #field: s.into() } }
    } else {
        quote! { Self(s.into()) }
    };

    quote! {
        #[doc = #doc_comment]
        #[inline]
        pub fn new<S: Into<String> + AsRef<str>>(s: S) -> Result<Self, #validator::Error> {
            #validator::validate(s.as_ref())?;
            Ok(#create)
        }

        #[doc = #doc_comment_unsafe]
        #[inline]
        pub unsafe fn new_unchecked<S: Into<String>>(s: S) -> Self {
            #create
        }
    }
}

fn normalized_owned_creation(
    ident: &syn::Ident,
    field: &Option<syn::Ident>,
    normalizer: &syn::Type,
) -> proc_macro2::TokenStream {
    let normalizer_tokens = normalizer.to_token_stream();
    let doc_comment = format!(
        "Constructs a new {} if it conforms to [`{}`] and normalizes the input",
        ident, normalizer_tokens
    );

    let doc_comment_unsafe = format!(
        "Constructs a new {} without validation or normalization\n\
        \n\
        ## Safety\n\
        \n\
        Consumers of this function must ensure that values conform to [`{}`] and \
        are in normalized form. Failure to maintain this invariant may lead to \
        undefined behavior.",
        ident, normalizer_tokens
    );

    let normalizer = super::as_normalizer(normalizer);

    let create = if let Some(field) = field {
        quote! { Self { #field: s.into() } }
    } else {
        quote! { Self(s.into()) }
    };

    let create_cow = if let Some(field) = field {
        quote! { Self { #field: result.into_owned() } }
    } else {
        quote! { Self(result.into_owned()) }
    };

    quote! {
        #[doc = #doc_comment]
        #[inline]
        pub fn new<S: AsRef<str>>(s: S) -> Result<Self, #normalizer::Error> {
            let result = #normalizer::normalize(s.as_ref())?;
            Ok(#create_cow)
        }

        #[doc = #doc_comment_unsafe]
        #[inline]
        pub unsafe fn new_unchecked<S: Into<String>>(s: S) -> Self {
            #create
        }
    }
}

fn inherent_impls(
    name: &syn::Ident,
    ref_type: &syn::Type,
    wrapped_type: &syn::Type,
    check_mode: &CheckMode,
    field_ident: &Option<syn::Ident>,
    field: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let creation_functions = match check_mode {
        CheckMode::None => infallible_owned_creation(name, field_ident),
        CheckMode::Validate(validator) => fallible_owned_creation(name, field_ident, validator),
        CheckMode::Normalize(normalizer) => normalized_owned_creation(name, field_ident, normalizer),
    };

    let doc_box = format!(
        "\
        Converts this `{}` into a [`Box`]`<`[`{}`]`>`\n\
        \n\
        This will drop any excess capacity.",
        name,
        ref_type.to_token_stream(),
    );
    let doc = format!(
        "Unwraps the underlying [`{}`] value",
        wrapped_type.to_token_stream()
    );

    quote! {
        impl #name {
            #creation_functions

            #[doc = #doc_box]
            #[inline]
            #[allow(unsafe_code)]
            pub fn into_boxed_ref(self) -> Box<#ref_type> {
                // SAFETY: A Box<str> has the same representation as a Box<#ref_type>.
                // Lifetimes are not implicated as the value on the heap is owned, so
                // this transmute is safe.
                let box_str = self.#field.into_boxed_str();
                unsafe { ::std::mem::transmute(box_str) }
            }

            #[doc = #doc]
            #[inline]
            pub fn into_string(self) -> #wrapped_type {
                self.#field
            }
        }
    }
}

fn construct_ref_item(
    name: &syn::Ident,
    vis: &syn::Visibility,
    ref_type: &syn::Type,
    field: &Option<syn::Ident>,
    attrs_vec: &[syn::Attribute],
    params: crate::borrow::Parameters,
    ref_doc: Option<String>,
) -> Result<proc_macro2::TokenStream, syn::Error> {
    let ref_vis = vis.clone();

    let ref_doc = ref_doc.unwrap_or_else(|| format!("A reference to a borrowed [`{}`]", name));

    let mut attrs = proc_macro2::TokenStream::new();
    attrs.append_all(attrs_vec);

    let items = if let Some(field) = field {
        quote! ( { #attrs #field: str } )
    } else {
        quote! { (#attrs str); }
    };

    crate::borrow::typed_string_ref_params(
        params,
        syn::parse_quote! {
                #[doc = #ref_doc]
                #ref_vis struct #ref_type #items
        },
    )
}

pub fn display_impl(name: &syn::Ident, ref_type: &syn::Type) -> proc_macro2::TokenStream {
    quote! {
        impl<'a> ::std::fmt::Display for #name {
            #[inline]
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                <#ref_type as ::std::fmt::Display>::fmt(::std::borrow::Borrow::borrow(self), f)
            }
        }
    }
}

pub fn debug_impl(name: &syn::Ident, ref_type: &syn::Type) -> proc_macro2::TokenStream {
    quote! {
        impl<'a> ::std::fmt::Debug for #name {
            #[inline]
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                <#ref_type as ::std::fmt::Debug>::fmt(::std::borrow::Borrow::borrow(self), f)
            }
        }
    }
}

pub fn common_impls(name: &syn::Ident, ref_type: &syn::Type, field: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    quote! {
        impl ::std::hash::Hash for #name {
            #[inline]
            fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
                use ::std::hash::Hash;

                self.#field.hash(state);
            }
        }

        impl ::std::cmp::Eq for #name {}

        impl PartialEq<#name> for #name {
            #[inline]
            fn eq(&self, other: &#name) -> bool {
                self.as_str() == other.as_str()
            }
        }

        impl From<&'_ #ref_type> for #name {
            #[inline]
            fn from(s: &#ref_type) -> Self {
                s.to_owned()
            }
        }

        impl ::std::borrow::Borrow<#ref_type> for #name {
            #[inline]
            fn borrow(&self) -> &#ref_type {
                ::std::ops::Deref::deref(self)
            }
        }

        impl AsRef<#ref_type> for #name {
            #[inline]
            fn as_ref(&self) -> &#ref_type {
                ::std::ops::Deref::deref(self)
            }
        }

        impl AsRef<str> for #name {
            #[inline]
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }
    }
}

// fn self_create(field: Option<syn::Ident>) -> impl Fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream {
//     move |value| {
//         if let Some(field) = field {
//             quote! { Self { #field: value } }
//         } else {
//             quote! { Self::new }
//         }
//     }
// }
//
fn infallible_conversion_impls(
    name: &syn::Ident,
    ref_type: &syn::Type,
    wrapped_type: &syn::Type,
    field: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    quote! {
        impl From<#wrapped_type> for #name {
            #[inline]
            fn from(s: #wrapped_type) -> Self {
                Self::new(s)
            }
        }

        impl From<&'_ str> for #name {
            #[inline]
            fn from(s: &str) -> Self {
                Self::new(#wrapped_type::from(s))
            }
        }

        impl ::std::str::FromStr for #name {
            type Err = ::std::convert::Infallible;

            #[inline]
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(Self::from(s))
            }
        }

        impl ::std::borrow::Borrow<str> for #name {
            #[inline]
            fn borrow(&self) -> &str {
                self.as_str()
            }
        }

        impl ::std::ops::Deref for #name {
            type Target = #ref_type;

            #[inline]
            fn deref(&self) -> &Self::Target {
                #ref_type::from_str(self.#field.as_str())
            }
        }
    }
}

fn fallible_conversion_impls(
    name: &syn::Ident,
    ref_type: &syn::Type,
    wrapped_type: &syn::Type,
    validator: &syn::Type,
    field: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let validator = super::as_validator(validator);

    quote! {
        impl ::std::convert::TryFrom<#wrapped_type> for #name {
            type Error = #validator::Error;

            #[inline]
            fn try_from(s: #wrapped_type) -> Result<Self, Self::Error> {
                Self::new(s)
            }
        }

        impl ::std::convert::TryFrom<&'_ str> for #name {
            type Error = #validator::Error;

            #[inline]
            fn try_from(s: &str) -> Result<Self, Self::Error> {
                Self::new(s)
            }
        }

        impl ::std::str::FromStr for #name {
            type Err = #validator::Error;

            #[inline]
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Self::new(s)
            }
        }

        impl ::std::borrow::Borrow<str> for #name {
            #[inline]
            fn borrow(&self) -> &str {
                self.as_str()
            }
        }

        impl ::std::ops::Deref for #name {
            type Target = #ref_type;

            #[inline]
            #[allow(unsafe_code)]
            fn deref(&self) -> &Self::Target {
                // SAFETY: At this point, we are certain that the underlying string
                // slice passes validation, so the implicit contract is satisfied.
                unsafe { #ref_type::from_str_unchecked(self.#field.as_str()) }
            }
        }
    }
}

fn normalized_conversion_impls(
    name: &syn::Ident,
    ref_type: &syn::Type,
    wrapped_type: &syn::Type,
    normalizer: &syn::Type,
    field: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let normalizer = super::as_normalizer(normalizer);

    quote! {
        impl ::std::convert::TryFrom<#wrapped_type> for #name {
            type Error = #normalizer::Error;

            #[inline]
            fn try_from(s: #wrapped_type) -> Result<Self, Self::Error> {
                Self::new(s)
            }
        }

        impl ::std::convert::TryFrom<&'_ str> for #name {
            type Error = #normalizer::Error;

            #[inline]
            fn try_from(s: &str) -> Result<Self, Self::Error> {
                Self::new(s)
            }
        }

        impl ::std::str::FromStr for #name {
            type Err = #normalizer::Error;

            #[inline]
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Self::new(s)
            }
        }

        impl ::std::ops::Deref for #name {
            type Target = #ref_type;

            #[inline]
            #[allow(unsafe_code)]
            fn deref(&self) -> &Self::Target {
                // SAFETY: At this point, we are certain that the underlying string
                // slice passes validation, so the implicit contract is satisfied.
                unsafe { #ref_type::from_str_unchecked(self.#field.as_str()) }
            }
        }
    }
}

fn conversion_impls(
    name: &syn::Ident,
    ref_type: &syn::Type,
    wrapped_type: &syn::Type,
    check_mode: &CheckMode,
    field: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let impls = match check_mode {
        CheckMode::None => infallible_conversion_impls(name, ref_type, wrapped_type, field),
        CheckMode::Validate(validator) => {
            fallible_conversion_impls(name, ref_type, wrapped_type, validator, field)
        }
        CheckMode::Normalize(normalizer) => {
            normalized_conversion_impls(name, ref_type, wrapped_type, normalizer, field)
        }
    };

    quote! {
        #impls

        impl From<#name> for Box<#ref_type> {
            #[inline]
            fn from(r: #name) -> Self {
                r.into_boxed_ref()
            }
        }

        impl From<Box<#ref_type>> for #name {
            #[inline]
            fn from(r: Box<#ref_type>) -> Self {
                r.into_owned()
            }
        }

        impl<'a> From<::std::borrow::Cow<'a, #ref_type>> for #name {
            #[inline]
            fn from(r: ::std::borrow::Cow<'a, #ref_type>) -> Self {
                match r {
                    ::std::borrow::Cow::Borrowed(b) => b.to_owned(),
                    ::std::borrow::Cow::Owned(o) => o,
                }
            }
        }

        impl<'a> From<#name> for ::std::borrow::Cow<'a, #ref_type> {
            #[inline]
            fn from(owned: #name) -> Self {
                ::std::borrow::Cow::Owned(owned)
            }
        }
    }
}

fn fallible_serde_tokens() -> proc_macro2::TokenStream {
    quote! {.map_err(<D::Error as ::serde::de::Error>::custom)?}
}

pub fn serde_impls(
    name: &syn::Ident,
    check_mode: &CheckMode,
    wrapped_type: &syn::Type,
    field: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let handle_failure = (!matches!(check_mode, CheckMode::None)).then(fallible_serde_tokens);

    quote! {
        impl ::serde::Serialize for #name {
            fn serialize<S: ::serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                <#wrapped_type as ::serde::Serialize>::serialize(&self.#field, serializer)
            }
        }

        #[allow(clippy::needless_question_mark)]
        impl<'de> ::serde::Deserialize<'de> for #name {
            fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let raw = <#wrapped_type as ::serde::Deserialize<'de>>::deserialize(deserializer)?;
                Ok(Self::new(raw)#handle_failure)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use quote::format_ident;
    use syn::parse_quote;

    fn owned_ident() -> syn::Ident {
        format_ident!("Owned")
    }

    fn validating_type() -> syn::Type {
        parse_quote! { TheValidator }
    }

    // fn borrowed_type() -> syn::Type {
    //     parse_quote!{ Borrowed }
    // }

    fn wrapped_type() -> syn::Type {
        parse_quote! { Wrapped }
    }

    #[test]
    fn expected_serde_impls_infallible() {
        let name = owned_ident();
        let wrapped: syn::Type = wrapped_type();

        let actual = serde_impls(&name, &CheckMode::None, &wrapped, &quote!{0});
        let expected = quote! {
            impl ::serde::Serialize for Owned {
                fn serialize<S: ::serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                    <Wrapped as ::serde::Serialize>::serialize(&self.0, serializer)
                }
            }

            #[allow(clippy::needless_question_mark)]
            impl<'de> ::serde::Deserialize<'de> for Owned {
                fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                    let raw = <Wrapped as ::serde::Deserialize<'de>>::deserialize(deserializer)?;
                    Ok(Self::new(raw))
                }
            }
        };

        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn expected_serde_impls_named_infallible() {
        let name = owned_ident();
        let wrapped: syn::Type = wrapped_type();

        let actual = serde_impls(&name, &CheckMode::None, &wrapped, &quote!{orange});
        let expected = quote! {
            impl ::serde::Serialize for Owned {
                fn serialize<S: ::serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                    <Wrapped as ::serde::Serialize>::serialize(&self.orange, serializer)
                }
            }

            #[allow(clippy::needless_question_mark)]
            impl<'de> ::serde::Deserialize<'de> for Owned {
                fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                    let raw = <Wrapped as ::serde::Deserialize<'de>>::deserialize(deserializer)?;
                    Ok(Self::new(raw))
                }
            }
        };

        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn expected_serde_impls_fallible() {
        let name = owned_ident();
        let wrapped: syn::Type = wrapped_type();

        let actual = serde_impls(&name, &CheckMode::Validate(validating_type()), &wrapped, &quote!{0});
        let expected = quote! {
            impl ::serde::Serialize for Owned {
                fn serialize<S: ::serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                    <Wrapped as ::serde::Serialize>::serialize(&self.0, serializer)
                }
            }

            #[allow(clippy::needless_question_mark)]
            impl<'de> ::serde::Deserialize<'de> for Owned {
                fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                    let raw = <Wrapped as ::serde::Deserialize<'de>>::deserialize(deserializer)?;
                    Ok(Self::new(raw).map_err(<D::Error as ::serde::de::Error>::custom)?)
                }
            }
        };

        assert_eq!(expected.to_string(), actual.to_string());
    }
}
