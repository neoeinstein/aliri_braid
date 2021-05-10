use crate::symbol::*;
use crate::{parse_lit_into_string, parse_lit_into_type};
use quote::ToTokens;

#[derive(Default)]
pub struct Parameters {
    pub ref_type: Option<syn::Type>,
    pub ref_doc: Option<String>,
    pub validate: bool,
    pub validator: Option<syn::Type>,
    pub derive_serde: bool,
    pub no_auto_ref: bool,
}

impl std::convert::TryFrom<syn::AttributeArgs> for Parameters {
    type Error = syn::Error;

    fn try_from(args: syn::AttributeArgs) -> Result<Self, Self::Error> {
        let mut params = Parameters::default();

        for arg in args {
            match arg {
                syn::NestedMeta::Meta(syn::Meta::NameValue(nv)) if nv.path == REF => {
                    params.ref_type = Some(parse_lit_into_type(REF, &nv.lit)?);
                }
                syn::NestedMeta::Meta(syn::Meta::NameValue(nv)) if nv.path == VALIDATOR => {
                    params.validate = true;
                    params.validator = Some(parse_lit_into_type(VALIDATOR, &nv.lit)?);
                }
                syn::NestedMeta::Meta(syn::Meta::NameValue(nv)) if nv.path == REF_DOC => {
                    params.ref_doc = Some(parse_lit_into_string(REF_DOC, &nv.lit)?);
                }
                syn::NestedMeta::Meta(syn::Meta::Path(p)) if p == SERDE => {
                    params.derive_serde = true;
                }
                syn::NestedMeta::Meta(syn::Meta::Path(p)) if p == NO_AUTO_REF => {
                    params.no_auto_ref = true;
                }
                syn::NestedMeta::Meta(syn::Meta::Path(p)) if p == VALIDATOR => {
                    params.validate = true;
                }
                syn::NestedMeta::Meta(syn::Meta::NameValue(syn::MetaNameValue {
                    ref path,
                    ..
                }))
                | syn::NestedMeta::Meta(syn::Meta::Path(ref path)) => {
                    return Err(syn::Error::new_spanned(
                        &arg,
                        format!(
                            "unsupported argument `{}`",
                            path.to_token_stream().to_string()
                        ),
                    ));
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        &arg,
                        "unsupported argument".to_string(),
                    ));
                }
            }
        }

        Ok(params)
    }
}
