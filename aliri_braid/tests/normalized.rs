use aliri_braid::braid;
use std::borrow::Cow;
use std::{error, fmt};

#[derive(Debug)]
pub struct EmptyString;

impl fmt::Display for EmptyString {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("string cannot be empty")
    }
}

impl error::Error for EmptyString {}

/// A non-empty [`String`] normalized to lowercase
#[braid(
    serde,
    normalizer,
    ref = "LowerStr",
    ref_doc = "A borrowed reference to a non-empty, lowercase string"
)]
pub struct LowerString;

impl aliri_braid::Normalizer for LowerString {
    type Error = EmptyString;

    fn normalize(s: &str) -> Result<Cow<str>, Self::Error> {
        if s.is_empty() {
            Err(EmptyString)
        } else if s.contains(|c: char| c.is_uppercase()) {
            Ok(Cow::Owned(s.to_lowercase()))
        } else {
            Ok(Cow::Borrowed(s))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owned_handles_already_normal() {
        let x = LowerString::new("testing").unwrap();
        assert_eq!(x.as_str(), "testing");
    }

    #[test]
    fn owned_handles_valid_non_normal() {
        let x = LowerString::new("TestIng").unwrap();
        assert_eq!(x.as_str(), "testing");
    }

    #[test]
    fn owned_rejects_invalid() {
        let x = LowerString::new("");
        assert!(matches!(x, Err(_)));
    }

    #[test]
    fn ref_handles_already_normal() {
        let x = LowerStr::from_str("testing").unwrap();
        assert!(matches!(x, Cow::Borrowed(_)));
        assert_eq!(x.as_str(), "testing");
    }

    #[test]
    fn ref_handles_valid_non_normal() {
        let x = LowerStr::from_str("TestIng").unwrap();
        assert!(matches!(x, Cow::Owned(_)));
        assert_eq!(x.as_str(), "testing");
    }

    #[test]
    fn ref_rejects_invalid() {
        let x = LowerStr::from_str("");
        assert!(matches!(x, Err(_)));
    }

    #[test]
    fn ref_norm_handles_already_normal() {
        let x = LowerStr::from_normalized_str("testing").unwrap();
        assert_eq!(x.as_str(), "testing");
    }

    #[test]
    fn ref_norm_rejects_valid_non_normal() {
        let x = LowerStr::from_normalized_str("TestIng");
        assert!(matches!(x, Err(_)));
    }

    #[test]
    fn ref_norm_rejects_invalid() {
        let x = LowerStr::from_normalized_str("");
        assert!(matches!(x, Err(_)));
    }
}
