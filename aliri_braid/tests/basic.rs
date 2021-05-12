use aliri_braid::braid;

/// A basic example of a wrapper around a [`String`]
#[braid(
    serde,
    ref_doc = "A borrowed reference to a basic string slice wrapper"
)]
pub struct BasicExampleBuf;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owned_works() {
        let x = BasicExampleBuf::new("Testing the Buffer");
        assert_eq!(x.as_str(), "Testing the Buffer");
    }

    #[test]
    fn ref_works() {
        let x = BasicExample::from_str("Testing the Reference");
        assert_eq!(x.as_str(), "Testing the Reference");
    }
}
