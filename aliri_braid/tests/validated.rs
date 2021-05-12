use aliri_braid::braid;
use std::{error, fmt};

#[derive(Debug)]
pub enum InvalidScopeToken {
    EmptyString,
    InvalidCharacter { position: usize, value: u8 },
}

impl fmt::Display for InvalidScopeToken {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::EmptyString => f.write_str("scope cannot be empty"),
            Self::InvalidCharacter { position, value } => f.write_fmt(format_args!(
                "invalid scope character at position {}: {:02x}",
                position, value
            )),
        }
    }
}

impl error::Error for InvalidScopeToken {}

/// A scope token as defined in RFC6749, Section 3.3
#[braid(serde, validator, ref_doc = "A borrowed reference to a [`ScopeToken`]")]
pub struct ScopeToken;

impl aliri_braid::Validator for ScopeToken {
    type Error = InvalidScopeToken;

    fn validate(s: &str) -> Result<(), Self::Error> {
        if s.is_empty() {
            Err(InvalidScopeToken::EmptyString)
        } else if let Some((position, &value)) = s
            .as_bytes()
            .iter()
            .enumerate()
            .find(|(_, &b)| b <= 0x20 || b == 0x22 || b == 0x5C)
        {
            Err(InvalidScopeToken::InvalidCharacter { position, value })
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owned_handles_valid() {
        let x = ScopeToken::new("https://crates.io/scopes/publish:crate").unwrap();
        assert_eq!(x.as_str(), "https://crates.io/scopes/publish:crate");
    }

    #[test]
    fn owned_rejects_empty() {
        let x = ScopeToken::new("");
        assert!(matches!(x, Err(InvalidScopeToken::EmptyString)));
    }

    #[test]
    fn owned_rejects_invalid() {
        let x = ScopeToken::new("https://crates.io/scopes/\"publish:crate\"");
        assert!(matches!(x, Err(InvalidScopeToken::InvalidCharacter { .. })));
    }

    #[test]
    fn ref_handles_valid() {
        let x = ScopeTokenRef::from_str("https://crates.io/scopes/publish:crate").unwrap();
        assert_eq!(x.as_str(), "https://crates.io/scopes/publish:crate");
    }

    #[test]
    fn ref_rejects_empty() {
        let x = ScopeTokenRef::from_str("");
        assert!(matches!(x, Err(InvalidScopeToken::EmptyString)));
    }

    #[test]
    fn ref_rejects_invalid() {
        let x = ScopeTokenRef::from_str("https://crates.io/scopes/\"publish:crate\"");
        assert!(matches!(x, Err(InvalidScopeToken::InvalidCharacter { .. })));
    }
}
