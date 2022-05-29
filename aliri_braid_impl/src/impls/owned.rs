mod parameters;

use crate::{check_mode::CheckMode, Field, FieldName};
pub use parameters::Parameters;
use quote::{format_ident, quote, ToTokens, TokenStreamExt};
use std::convert::TryInto;

use self::parameters::AttrList;

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
    let name = &body.ident;

    let inherent_impls = config.inherent();
    let common_impls = common_impls(name, &ref_type);
    let conversion_impls = conversion_impls(name, &ref_type, wrapped_type, &check_mode, &field.name);

    let serde_impls = config.impls.owned_serde_impl(name, &check_mode, &config.field);

    let body_attrs = &body.attrs;
    //let construct_ref_item = (!no_auto_ref)
    let construct_ref_item = true
        .then(|| {
            construct_ref_item(
                name,
                &vis,
                &ref_type,
                &field,
                body_attrs,
                &ref_attrs,
                crate::borrow::Parameters {
                    owned_type: Some(syn::parse_quote!(#name)),
                    check_mode,
                    omit_debug: impl_debug != parameters::DelegatingImplOption::Auto,
                    omit_display: impl_display != parameters::DelegatingImplOption::Auto,
                    derive_serde,
                },
                ref_doc,
            )
        })
        .transpose()?;


    let debug_impl =
        (impl_debug != parameters::DelegatingImplOption::None).then(|| debug_impl(name, &ref_type));
    let display_impl =
        (impl_debug != parameters::DelegatingImplOption::None).then(|| display_impl(name, &ref_type));

    let owned_attr: proc_macro2::TokenStream = owned_attrs.into_iter().map(|a| quote!{#[#a]}).collect();
    let output = quote! {
        #clone
        #[derive(Hash, PartialEq, Eq, PartialOrd, Ord)]
        #[repr(transparent)]
        #owned_attr
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


#[allow(clippy::too_many_arguments)]
fn construct_ref_item(
    name: &syn::Ident,
    vis: &syn::Visibility,
    ref_type: &syn::Type,
    field: &Field,
    body_attrs: &[syn::Attribute],
    ref_attrs: &AttrList,
    params: crate::borrow::Parameters,
    ref_doc: Vec<String>,
) -> Result<proc_macro2::TokenStream, syn::Error> {
    let ref_vis = vis.clone();

    let doc_attrs: proc_macro2::TokenStream = ref_doc.into_iter().map(|d| quote!{ #[doc = #d] }).collect();
    let attrs: proc_macro2::TokenStream = body_attrs.iter().map(|a| quote!{ #a }).chain(attr_list_to_attr_token_stream(ref_attrs)).collect();

    let mut field_attrs = proc_macro2::TokenStream::new();
    field_attrs.append_all(field.attrs);

    crate::borrow::typed_string_ref_params(
        params,
        syn::parse_quote! {
                #doc_attrs
                #attrs
                #ref_vis struct #ref_type #body
        },
    )
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

        let actual = serde_impls(&name, &CheckMode::None, &wrapped, &FieldName::Unnamed);
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

        let actual = serde_impls(&name, &CheckMode::None, &wrapped, &FieldName::Named(&syn::Ident::new("orange", proc_macro2::Span::call_site())));
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

        let actual = serde_impls(
            &name,
            &CheckMode::Validate(validating_type()),
            &wrapped,
            &FieldName::Unnamed,
        );
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

fn attr_list_to_attr_token_stream(list: &AttrList) -> impl Iterator<Item=proc_macro2::TokenStream> + '_ {
    list.iter().map(|a| quote!{#[#a]})
}
