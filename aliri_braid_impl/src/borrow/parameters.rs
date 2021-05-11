use crate::check_mode::CheckMode;
use crate::parse_lit_into_type;
use crate::symbol::*;
use quote::ToTokens;

#[derive(Default)]
pub struct Parameters {
    pub owned_type: Option<syn::Type>,
    pub check_mode: CheckMode,
    pub derive_serde: bool,
}

impl std::convert::TryFrom<syn::AttributeArgs> for Parameters {
    type Error = syn::Error;

    fn try_from(args: syn::AttributeArgs) -> Result<Self, Self::Error> {
        let mut params = Parameters::default();

        for arg in &args {
            match arg {
                syn::NestedMeta::Meta(syn::Meta::NameValue(nv)) if nv.path == OWNED => {
                    params.owned_type = Some(parse_lit_into_type(OWNED, &nv.lit)?);
                }
                syn::NestedMeta::Meta(syn::Meta::NameValue(nv)) if nv.path == VALIDATOR => {
                    let validator = parse_lit_into_type(VALIDATOR, &nv.lit)?;
                    params
                        .check_mode
                        .try_set_validator(validator)
                        .map_err(|s| syn::Error::new_spanned(arg, s))?;
                }
                syn::NestedMeta::Meta(syn::Meta::NameValue(nv)) if nv.path == NORMALIZER => {
                    let normalizer = parse_lit_into_type(NORMALIZER, &nv.lit)?;
                    params
                        .check_mode
                        .try_set_normalizer(normalizer)
                        .map_err(|s| syn::Error::new_spanned(arg, s))?;
                }
                syn::NestedMeta::Meta(syn::Meta::Path(p)) if p == SERDE => {
                    params.derive_serde = true;
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
