//! You probably want the [`aliri_braid`] crate, which
//! has the documentation this crate lacks.
//!
//!   [`aliri_braid`]: https://docs.rs/aliri_braid/*/aliri_braid/

#![warn(
    missing_docs,
    unused_import_braces,
    unused_imports,
    unused_qualifications
)]
#![deny(
    missing_debug_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unused_must_use
)]
#![forbid(unsafe_code)]

extern crate proc_macro;

mod borrow;
mod check_mode;
mod owned;
mod symbol;

use symbol::*;

use proc_macro::TokenStream;
use syn::parse_macro_input;

/// Constructs a braid
///
/// Available options:
/// * `ref = "RefName"`
///   * Sets the name of the borrowed type
/// * `ref_doc = "Alternate doc comment"`
///   * Overrides the default doc comment for the borrowed type
/// * either `validator [ = "Type" ]` or `normalizer [ = "Type" ]`
///   * Indicates the type is validated or normalized. If not specified,
///     it is assumed that the braid implements the relevant trait itself.
/// * `omit_clone`
///   * Prevents the owned type from automatically deriving a `Clone` implementation
/// * `debug_impl = "auto|owned|none"` (default `auto`)
///   * Changes how automatic implementations of the `Debug` trait are provided.
///     If `owned`, then the owned type will generate a `Debug` implementation that
///     will just delegate to the borrowed implementation.
///     If `none`, then no implementations of `Debug` will be provided.
/// * `display_impl = "auto|owned|none"` (default `auto`)
///   * Changes how automatic implementations of the `Display` trait are provided.
///     If `owned`, then the owned type will generate a `Display` implementation that
///     will just delegate to the borrowed implementation.
///     If `none`, then no implementations of `Display` will be provided.
/// * `serde`
///   * Adds serialize and deserialize implementations
#[proc_macro_attribute]
pub fn braid(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as syn::AttributeArgs);
    let body = parse_macro_input!(input as syn::ItemStruct);

    owned::typed_string_tokens(args, body)
        .unwrap_or_else(|e| e.into_compile_error())
        .into()
}

#[proc_macro_attribute]
#[doc(hidden)]
pub fn braid_ref(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as syn::AttributeArgs);
    let body = parse_macro_input!(input as syn::ItemStruct);

    borrow::typed_string_ref_tokens(args, body)
        .unwrap_or_else(|e| e.into_compile_error())
        .into()
}

fn as_validator(validator: &syn::Type) -> proc_macro2::TokenStream {
    quote::quote! { <#validator as ::aliri_braid::Validator> }
}

fn as_normalizer(normalizer: &syn::Type) -> proc_macro2::TokenStream {
    quote::quote! { <#normalizer as ::aliri_braid::Normalizer> }
}

fn get_lit_str(attr_name: Symbol, lit: &syn::Lit) -> Result<&syn::LitStr, syn::Error> {
    if let syn::Lit::Str(lit) = lit {
        Ok(lit)
    } else {
        Err(syn::Error::new_spanned(
            lit,
            format!(
                "expected attribute `{}` to have a string value (`{} = \"value\"`)",
                attr_name, attr_name
            ),
        ))
    }
}

// fn parse_lit_into_path(attr_name: Symbol, lit: &syn::Lit) -> Result<syn::Path, ()> {
//     let string = get_lit_str( attr_name, lit)?;
//     parse_lit_str(string).map_err(|_| {
//         syn::Error::new_spanned(lit, format!("failed to parse path: {:?}", string.value()))
//     })
// }

fn parse_lit_into_type(attr_name: Symbol, lit: &syn::Lit) -> Result<syn::Type, syn::Error> {
    let string = get_lit_str(attr_name, lit)?;
    parse_lit_str(string).map_err(|_| {
        syn::Error::new_spanned(lit, format!("failed to parse type: {:?}", string.value()))
    })
}

fn parse_lit_into_string(attr_name: Symbol, lit: &syn::Lit) -> Result<String, syn::Error> {
    let string = get_lit_str(attr_name, lit)?;
    Ok(string.value())
}

fn parse_lit_str<T>(s: &syn::LitStr) -> syn::parse::Result<T>
where
    T: syn::parse::Parse,
{
    let tokens = spanned_tokens(s)?;
    syn::parse2(tokens)
}

fn spanned_tokens(s: &syn::LitStr) -> syn::parse::Result<proc_macro2::TokenStream> {
    let stream = syn::parse_str(&s.value())?;
    Ok(respan_token_stream(stream, s.span()))
}

fn respan_token_stream(
    stream: proc_macro2::TokenStream,
    span: proc_macro2::Span,
) -> proc_macro2::TokenStream {
    stream
        .into_iter()
        .map(|token| respan_token_tree(token, span))
        .collect()
}

fn respan_token_tree(
    mut token: proc_macro2::TokenTree,
    span: proc_macro2::Span,
) -> proc_macro2::TokenTree {
    if let proc_macro2::TokenTree::Group(g) = &mut token {
        *g = proc_macro2::Group::new(g.delimiter(), respan_token_stream(g.stream(), span));
    }
    token.set_span(span);
    token
}
