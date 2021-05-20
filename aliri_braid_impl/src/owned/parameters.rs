use crate::check_mode::IndefiniteCheckMode;
use crate::symbol::*;
use crate::{parse_lit_into_string, parse_lit_into_type};
use quote::ToTokens;

#[derive(Default)]
pub struct Parameters {
    pub ref_type: Option<syn::Type>,
    pub ref_doc: Option<String>,
    pub check_mode: IndefiniteCheckMode,
    pub omit_clone: bool,
    pub impl_debug: AutoImplOption,
    pub impl_display: AutoImplOption,
    pub derive_serde: bool,
    pub no_auto_ref: bool,
}

impl std::convert::TryFrom<syn::AttributeArgs> for Parameters {
    type Error = syn::Error;

    fn try_from(args: syn::AttributeArgs) -> Result<Self, Self::Error> {
        let mut params = Parameters::default();

        for arg in &args {
            match arg {
                syn::NestedMeta::Meta(syn::Meta::NameValue(nv)) if nv.path == REF => {
                    params.ref_type = Some(parse_lit_into_type(REF, &nv.lit)?);
                }
                syn::NestedMeta::Meta(syn::Meta::NameValue(nv)) if nv.path == VALIDATOR => {
                    let validator = parse_lit_into_type(VALIDATOR, &nv.lit)?;
                    params
                        .check_mode
                        .try_set_validator(Some(validator))
                        .map_err(|s| syn::Error::new_spanned(arg, s))?;
                }
                syn::NestedMeta::Meta(syn::Meta::NameValue(nv)) if nv.path == NORMALIZER => {
                    let normalizer = parse_lit_into_type(NORMALIZER, &nv.lit)?;
                    params
                        .check_mode
                        .try_set_normalizer(Some(normalizer))
                        .map_err(|s| syn::Error::new_spanned(arg, s))?;
                }
                syn::NestedMeta::Meta(syn::Meta::NameValue(nv)) if nv.path == REF_DOC => {
                    params.ref_doc = Some(parse_lit_into_string(REF_DOC, &nv.lit)?);
                }
                syn::NestedMeta::Meta(syn::Meta::NameValue(nv)) if nv.path == DEBUG_IMPL => {
                    params.impl_debug = parse_lit_into_string(REF_DOC, &nv.lit)?.parse::<AutoImplOption>().map_err(|e| syn::Error::new_spanned(&arg, e.to_owned()))?;
                }
                syn::NestedMeta::Meta(syn::Meta::NameValue(nv)) if nv.path == DISPLAY_IMPL => {
                    params.impl_display = parse_lit_into_string(REF_DOC, &nv.lit)?.parse::<AutoImplOption>().map_err(|e| syn::Error::new_spanned(&arg, e.to_owned()))?;
                }
                syn::NestedMeta::Meta(syn::Meta::Path(p)) if p == SERDE => {
                    params.derive_serde = true;
                }
                syn::NestedMeta::Meta(syn::Meta::Path(p)) if p == NO_AUTO_REF => {
                    params.no_auto_ref = true;
                }
                syn::NestedMeta::Meta(syn::Meta::Path(p)) if p == OMIT_CLONE => {
                    params.omit_clone = true;
                }
                syn::NestedMeta::Meta(syn::Meta::Path(p)) if p == VALIDATOR => {
                    params
                        .check_mode
                        .try_set_validator(None)
                        .map_err(|s| syn::Error::new_spanned(arg, s))?;
                }
                syn::NestedMeta::Meta(syn::Meta::Path(p)) if p == NORMALIZER => {
                    params
                        .check_mode
                        .try_set_normalizer(None)
                        .map_err(|s| syn::Error::new_spanned(arg, s))?;
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AutoImplOption {
    Auto,
    OwnedOnly,
    None,
}

impl Default for AutoImplOption {
    fn default() -> Self {
        Self::Auto
    }
}

impl std::str::FromStr for AutoImplOption {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "auto" => Ok(Self::Auto),
            "owned" => Ok(Self::OwnedOnly),
            "none" => Ok(Self::None),
            _ => Err("valid values are: `auto`, `owned`, or `none`")
        }
    }
}
