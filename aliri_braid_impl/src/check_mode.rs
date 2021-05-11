use crate::symbol::*;
use quote::ToTokens;

pub enum CheckMode {
    None,
    Validate(syn::Type),
    Normalize(syn::Type),
}

impl Default for CheckMode {
    fn default() -> Self {
        Self::None
    }
}

impl CheckMode {
    pub fn try_set_validator(&mut self, validator: syn::Type) -> Result<(), String> {
        if matches!(self, Self::None) {
            *self = Self::Validate(validator);
            return Ok(());
        }

        let err_desc = if matches!(self, Self::Validate(_)) {
            format!("{} can only be specified once", VALIDATOR)
        } else {
            format!(
                "only one of {} and {} can be specified at a time",
                VALIDATOR, NORMALIZER,
            )
        };

        Err(err_desc)
    }

    pub fn try_set_normalizer(&mut self, normalizer: syn::Type) -> Result<(), String> {
        if matches!(self, Self::None) {
            *self = Self::Normalize(normalizer);
            return Ok(());
        }

        let err_desc = if matches!(self, Self::Normalize(_)) {
            format!("{} can only be specified once", NORMALIZER)
        } else {
            format!(
                "only one of {} and {} can be specified at a time",
                VALIDATOR, NORMALIZER,
            )
        };

        Err(err_desc)
    }
}

pub enum IndefiniteCheckMode {
    None,
    Validate(Option<syn::Type>),
    Normalize(Option<syn::Type>),
}

impl Default for IndefiniteCheckMode {
    fn default() -> Self {
        Self::None
    }
}

impl IndefiniteCheckMode {
    pub fn try_set_validator(&mut self, validator: Option<syn::Type>) -> Result<(), String> {
        if matches!(self, Self::None) {
            *self = Self::Validate(validator);
            return Ok(());
        }

        let err_desc = if matches!(self, Self::Validate(_)) {
            format!("{} can only be specified once", VALIDATOR)
        } else {
            format!(
                "only one of {} and {} can be specified at a time",
                VALIDATOR, NORMALIZER,
            )
        };

        Err(err_desc)
    }

    pub fn try_set_normalizer(&mut self, normalizer: Option<syn::Type>) -> Result<(), String> {
        if matches!(self, Self::None) {
            *self = Self::Normalize(normalizer);
            return Ok(());
        }

        let err_desc = if matches!(self, Self::Normalize(_)) {
            format!("{} can only be specified once", NORMALIZER)
        } else {
            format!(
                "only one of {} and {} can be specified at a time",
                VALIDATOR, NORMALIZER,
            )
        };

        Err(err_desc)
    }

    pub fn infer_validator_if_missing(self, default: &syn::Ident) -> CheckMode {
        match self {
            Self::None => CheckMode::None,
            Self::Validate(Some(validator)) => CheckMode::Validate(validator),
            Self::Validate(None) => CheckMode::Validate(ident_to_type(default)),
            Self::Normalize(Some(normalizer)) => CheckMode::Normalize(normalizer),
            Self::Normalize(None) => CheckMode::Normalize(ident_to_type(default)),
        }
    }
}

pub fn ident_to_type(ident: &syn::Ident) -> syn::Type {
    let tokens = ident.to_token_stream();

    syn::parse_quote!(#tokens)
}
