use braid::braid;

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

impl braid::Validator for ValidatedBuf {
    type Error = InvalidData;
    fn validate(s: &str) -> Result<(), Self::Error> {
        if s.chars().any(|c| c.len_utf8() > 3) {
            Err(InvalidData)
        } else {
            Ok(())
        }
    }
}
