#![deny(unsafe_code)]

use std::borrow::Cow;

use aliri_braid::braid;

mod common;

#[braid]
pub struct Basic;

#[braid(ref = "SomeRefName")]
pub struct CustomRefName;

#[braid(validator = "ValidatedBuf")]
pub struct ExternallyValidated;

#[braid(ref = "SomeValidatedRefName", validator = "ValidatedBuf")]
pub struct ValidatedWithCustomRefName;

#[braid(serde)]
pub struct Orange;

#[braid(
    serde,
    validator,
    ref_doc = "A reference to a cool new orange, that isn't yours!"
)]
pub struct ValidatedBuf;

#[derive(Debug, PartialEq, Eq)]
pub struct InvalidData;

impl std::fmt::Display for InvalidData {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("found 4-byte UTF-8 codepoints")
    }
}
impl std::error::Error for InvalidData {}

impl aliri_braid::Validator for ValidatedBuf {
    type Error = InvalidData;
    fn validate(s: &str) -> Result<(), Self::Error> {
        if s.chars().any(|c| c.len_utf8() > 3) {
            Err(InvalidData)
        } else {
            Ok(())
        }
    }
}

#[braid(
    serde,
    normalizer,
    ref_doc = "A reference to a cool new orange, that isn't yours!"
)]
pub struct NormalizedBuf;

impl aliri_braid::Normalizer for NormalizedBuf {
    type Error = InvalidData;
    fn normalize(s: &str) -> Result<Cow<str>, Self::Error> {
        if s.chars().any(|c| c.len_utf8() > 3) {
            Err(InvalidData)
        } else if s.contains(' ') {
            Ok(Cow::Owned(s.replace(" ", "")))
        } else {
            Ok(Cow::Borrowed(s))
        }
    }
}
