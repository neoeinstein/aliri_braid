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

mod codegen;

use codegen::Params;
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
    let mut body = parse_macro_input!(input as syn::ItemStruct);

    Params::parse(&args)
        .and_then(|p| p.build(&mut body))
        .map_or_else(syn::Error::into_compile_error, |codegen| codegen.generate())
        .into()
}

fn as_validator(validator: &syn::Type) -> proc_macro2::TokenStream {
    quote::quote! { <#validator as ::aliri_braid::Validator> }
}

fn as_normalizer(normalizer: &syn::Type) -> proc_macro2::TokenStream {
    quote::quote! { <#normalizer as ::aliri_braid::Normalizer> }
}
