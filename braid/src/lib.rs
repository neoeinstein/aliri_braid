//! Improve and strengthen your strings
//!
//! Strongly-typed APIs reduce errors and confusion over passing around un-typed strings.
//! Braid helps in that endeavor by making it painless to create wrappers around your
//! string values, ensuring that you use them in the right way every time.
//!
//! ## Usage
//!
//! A braid is created by attaching `#[braid]` to a struct definition. The macro will take
//! care of automatically updating the representation of the struct to wrap a string and
//! generate the borrowed form of the strong type.
//!
//! ```
//! use braid::braid;
//!
//! #[braid]
//! pub struct DatabaseName;
//! ```
//!
//! Once created, braids can be passed around as strongly-typed strings.
//!
//! ```
//!# use braid::braid;
//!#
//! fn take_strong_string(n: DatabaseName) {}
//! fn borrow_strong_string(n: &DatabaseNameRef) {}
//!
//!# #[braid]
//!# pub struct DatabaseName;
//!#
//! let owned = DatabaseName::new("mongo");
//! borrow_strong_string(&owned);
//! take_strong_string(owned);
//! ```
//!
//! A braid can also be untyped for use in stringly-typed interfaces.
//!
//! ```
//!# use braid::braid;
//!#
//! fn take_raw_string(s: String) {}
//! fn borrow_raw_str(s: &str) {}
//!
//!# #[braid]
//!# pub struct DatabaseName;
//!#
//! let owned = DatabaseName::new("mongo");
//! borrow_raw_str(owned.as_str());
//! take_raw_string(owned.into_string());
//! ```
//!
//! By default, the name of the borrowed form will be the same as the owned form
//! with `Ref` appended to the end.
//!
//! ```
//!# use braid::braid;
//!#
//! #[braid]
//! pub struct DatabaseName;
//!
//! let owned = DatabaseName::new("mongo");
//! let borrowed = DatabaseNameRef::from_str("mongo");
//!# assert_eq!(owned, borrowed);
//! ```
//!
//! If the name ends with `Buf`, however, then the borrowed form will drop the `Buf`, similar
//! to the relationship between
//! [`PathBuf`][std::path::PathBuf] and [`Path`][std::path::Path].
//!
//! ```
//!# use braid::braid;
//!#
//! #[braid]
//! pub struct DatabaseNameBuf;
//!
//! let owned = DatabaseNameBuf::new("mongo");
//! let borrowed = DatabaseName::from_str("mongo");
//!# assert_eq!(owned, borrowed);
//! ```
//!
//! If a different name is desired, this behavior can be
//! overridden by specifying the name of the reference type to create using the `ref`
//! parameter.
//!
//! ```
//!# use braid::braid;
//!#
//! #[braid(ref = "TempDb")]
//! pub struct DatabaseNameBuf;
//!
//! let owned = DatabaseNameBuf::new("mongo");
//! let borrowed = TempDb::from_str("mongo");
//! let to_owned: DatabaseNameBuf = borrowed.to_owned();
//!# assert_eq!(owned, borrowed);
//! ```
//!
//! A default doc comment is added to the borrowed form that refers back to the owned form.
//! If a custom doc comment is desired, the `ref_doc` parameter allows supplying custom
//! documentation.
//!
//! ```
//!# use braid::braid;
//!#
//! #[braid(ref_doc = "A temporary reference to a database name")]
//! pub struct DatabaseName;
//!#
//!# let owned = DatabaseName::new("mongo");
//!# let borrowed = DatabaseNameRef::from_str("mongo");
//!# assert_eq!(owned, borrowed);
//! ```
//!
//! ## Extensibility
//!
//! The types created by the `braid` macro are placed in the same module where declared.
//! This means additional functionality, including mutations, can be implemented easily.
//!
//! As a basic example, here is a type built to hold Amazon ARNs. The type has been
//! extended to support some mutation and introspection.
//!
//! ```
//!# use braid::braid;
//!#
//! #[braid]
//! pub struct AmazonArnBuf;
//!
//! impl AmazonArnBuf {
//!     /// Append an ARN segment
//!     pub fn add_segment(&mut self, segment: &str) {
//!         self.0.push_str(":");
//!         self.0.push_str(segment);
//!     }
//! }
//!
//! impl AmazonArn {
//!     /// Returns an iterator of all ARN segments
//!     pub fn get_segments(&self) -> std::str::Split<char> {
//!         self.0.split(':')
//!     }
//!
//!     /// Returns the service segment of the ARN
//!     pub fn get_service(&self) -> &str {
//!         self.get_segments().nth(2).unwrap_or("")
//!     }
//! }
//! ```
//!
//! ## Encapsulation
//!
//! Because code within the same module where the braid is defined are allowed to
//! access the internal value, you can use a module in order to more strictly
//! enforce encapsulation and limit accessibility that might otherwise violate
//! established invariants. This may be particularly desired when the wrapped type
//! requires [validation](#validation).
//!
//! ```
//! mod amazon_arn {
//!     #[braid::braid]
//!     pub struct AmazonArnBuf;
//!
//!     /* Additional impls that need access to the inner values */
//!#     impl AmazonArn {
//!#         pub fn get_segments(&self) -> std::str::Split<char> {
//!#             self.0.split(':')
//!#         }
//!#
//!#         pub fn get_service(&self) -> &str {
//!#             self.get_segments().nth(2).unwrap_or("")
//!#         }
//!#     }
//! }
//!
//! pub use amazon_arn::{AmazonArnBuf, AmazonArn};
//!
//! # fn main() {
//! let x = AmazonArnBuf::new("arn:aws:iam::123456789012:user/Development");
//! assert_eq!("iam", x.get_service());
//! # }
//! ```
//!
//! ## Soundness
//!
//! This crate ensures that the `from_str` implementation provided for wrapping
//! borrowed `str` slices does not extend lifetimes.
//!
//! In the example below, we verify that the borrowed `DatabaseNameRef` is unable
//! to escape the lifetime of `data`. The following code snippet will fail to
//! compile, because `data` will go out of scope and be dropped at the end of
//! the block creating `ex_ref`.
//!
//! ```compile_fail
//!# use braid::braid;
//!#
//!# #[braid]
//!# pub struct DatabaseName;
//!#
//! let ex_ref = {
//!     let data = DatabaseName::new("test string");
//!     DatabaseNameRef::from_str(data.as_str())
//! }; // `data` is dropped at this point
//!
//! // Which means that `ex_ref` would be invalid if allowed.
//! println!("{}", ex_ref);
//! ```
//!
//! ## Validation
//!
//! Types can be configured to only contain certain values. This can be used to strongly
//! enforce domain type boundaries, thus making invalid values unrepresentable.
//!
//! For example, if you wanted to have a username type that did not accept the `root` user,
//! you have a few options:
//!
//! 1. Pass the username around as a string, validate that it isn't `root` at known entry points.
//! 2. Create a username type and allow creation from a raw string, then validate it
//!    just after creation.
//! 3. Create a strong username type that requires the value to be validated prior to being
//!    creatable.
//!
//! Braided strings give the strongest, third guarantee. The other two methods require constant
//! vigilance to ensure that an unexpected `root` value doesn't sneak in through other backdoors.
//!
//! By default, Rust's module system allows items within the same module to have access to
//! each other's non-public members. If not handled properly, this can lead to unintentionally
//! violating invariants. Thus, for the strongest guarantees, it is recommended to use the module
//! system to further control access to the interior values held by the braided type as
//! described in the section on [encapsulation](#encapsulation).
//!
//! ```
//!# use braid::braid;
//!#
//! #[derive(Debug, PartialEq, Eq)]
//! pub struct InvalidUsername;
//! // Error implementation elided
//!# impl std::fmt::Display for InvalidUsername {
//!#     fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
//!#         f.write_str("invalid username")
//!#     }
//!# }
//!# impl std::error::Error for InvalidUsername {}
//!
//! #[braid(validator)]
//! pub struct NonRootUsername;
//!
//! impl braid::Validator for NonRootUsername {
//!     type Error = InvalidUsername;
//!     fn validate(s: &str) -> Result<(), Self::Error> {
//!         if s.is_empty() || s.eq_ignore_ascii_case("root") {
//!             Err(InvalidUsername)
//!         } else {
//!             Ok(())
//!         }
//!     }
//! }
//!
//! assert!(NonRootUsername::new("").is_err());
//! assert!(NonRootUsername::new("root").is_err());
//! assert!(NonRootUsername::new("nobody").is_ok());
//!
//! assert!(NonRootUsernameRef::from_str("").is_err());
//! assert!(NonRootUsernameRef::from_str("root").is_err());
//! assert!(NonRootUsernameRef::from_str("nobody").is_ok());
//! ```
//!
//! Foreign validators can also be used by specifying the name of the type that
//! implements the validation logic.
//!
//! ```
//!# use braid::braid;
//!#
//!# #[derive(Debug, PartialEq, Eq)]
//!# pub struct InvalidUsername;
//!# // Error implementation elided
//!# impl std::fmt::Display for InvalidUsername {
//!#     fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
//!#         f.write_str("invalid username")
//!#     }
//!# }
//!# impl std::error::Error for InvalidUsername {}
//!#
//! #[braid(validator = "UsernameValidator")]
//! pub struct NonRootUsername;
//!
//! pub struct UsernameValidator;
//!
//! impl braid::Validator for UsernameValidator {
//!     /* … */
//!#     type Error = InvalidUsername;
//!#     fn validate(s: &str) -> Result<(), Self::Error> {
//!#         if s.is_empty() || s.eq_ignore_ascii_case("root") {
//!#             Err(InvalidUsername)
//!#         } else {
//!#             Ok(())
//!#         }
//!#     }
//! }
//!
//! assert!(NonRootUsername::new("").is_err());
//! assert!(NonRootUsername::new("root").is_err());
//! assert!(NonRootUsername::new("nobody").is_ok());
//!
//! assert!(NonRootUsernameRef::from_str("").is_err());
//! assert!(NonRootUsernameRef::from_str("root").is_err());
//! assert!(NonRootUsernameRef::from_str("nobody").is_ok());
//! ```
//!
//! ### Unchecked creation
//!
//! A braided string, however, can only have that guarantee violated through the use of the
//! unsafe `new_unchecked()` function.
//!
//! ```compile_fail
//!# use braid::braid;
//!#
//!# #[derive(Debug, PartialEq, Eq)]
//!# pub struct InvalidUsername;
//!# // Error implementation elided
//!# impl std::fmt::Display for InvalidUsername {
//!#     fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
//!#         f.write_str("invalid username")
//!#     }
//!# }
//!# impl std::error::Error for InvalidUsername {}
//!#
//!# #[braid(validator)]
//!# pub struct NonRootUsername;
//!#
//!# impl braid::Validator for NonRootUsername {
//!#     type Error = InvalidUsername;
//!#     fn validate(s: &str) -> Result<(), Self::Error> {
//!#         if s.is_empty() || s.eq_ignore_ascii_case("root") {
//!#             Err(InvalidUsername)
//!#         } else {
//!#             Ok(())
//!#         }
//!#     }
//!# }
//!#
//! NonRootUsername::new_unchecked("");
//! NonRootUsernameRef::from_str_unchecked("nobody");
//! ```
//!
//! If you find violations of your guarantees, you can look specifically for uses of `unsafe`.
//!
//! ```
//!# use braid::braid;
//!#
//!# #[derive(Debug, PartialEq, Eq)]
//!# pub struct InvalidUsername;
//!# // Error implementation elided
//!# impl std::fmt::Display for InvalidUsername {
//!#     fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
//!#         f.write_str("invalid username")
//!#     }
//!# }
//!# impl std::error::Error for InvalidUsername {}
//!#
//!# #[braid(validator)]
//!# pub struct NonRootUsername;
//!#
//!# impl braid::Validator for NonRootUsername {
//!#     type Error = InvalidUsername;
//!#     fn validate(s: &str) -> Result<(), Self::Error> {
//!#         if s.is_empty() || s.eq_ignore_ascii_case("root") {
//!#             Err(InvalidUsername)
//!#         } else {
//!#             Ok(())
//!#         }
//!#     }
//!# }
//!#
//! unsafe {
//!     NonRootUsername::new_unchecked("");
//!     NonRootUsernameRef::from_str_unchecked("root");
//! }
//! ```
//!
//! This gives you the power to bypass validation through the use of `unsafe` when you can
//! guarantee the value is value.
//!
//! ## Provided trait impls
//!
//! By default, the following traits will be automatically implemented.
//!
//! For the `Owned` type
//! * [`std::clone::Clone`]
//! * [`std::fmt::Debug`]
//! * [`std::fmt::Display`]
//! * [`std::hash::Hash`]
//! * [`std::cmp::PartialEq<Owned>`]
//! * [`std::cmp::PartialEq<Borrowed>`]
//! * [`std::cmp::PartialEq<&Borrowed>`]
//! * [`std::cmp::PartialEq<Box<Borrowed>>`]
//! * [`std::cmp::Eq`]
//! * [`std::convert::AsRef<Borrowed>`]
//! * [`std::convert::AsRef<str>`]
//! * [`std::borrow::Borrow<Borrowed>`]
//! * [`std::ops::Deref`] where `Target = Borrowed`
//!
//! Additionally, unvalidated owned types implement
//! * [`std::convert::From<String>`]
//! * [`std::convert::From<&str>`]
//!
//! Validated owned types will instead implement
//! * [`std::convert::TryFrom<String>`]
//! * [`std::convert::TryFrom<&str>`]
//!
//! For the `Borrowed` type
//! * [`std::clone::Clone`]
//! * [`std::fmt::Debug`]
//! * [`std::fmt::Display`]
//! * [`std::hash::Hash`]
//! * [`std::cmp::PartialEq<Owned>`]
//! * [`std::cmp::PartialEq<Borrowed>`]
//! * [`std::cmp::PartialEq<&Borrowed>`]
//! * [`std::cmp::PartialEq<Box<Borrowed>>`]
//! * [`std::cmp::Eq`]
//! * [`std::borrow::ToOwned`] where `Owned = Owned`
//! * [`std::convert::AsRef<str>`]
//!
//! Additionally, unvalidated borrowed types implement
//! * [`std::convert::From<&str>`]
//!
//! Validated owned types will instead implement
//! * [`std::convert::TryFrom<&str>`]
//!
//! `Deref` to a `str` is explicitly not implemented. This means that an explicit call is
//! required to treat a value as an untyped string, whether `.as_str()`, `.to_string()`, or
//! `.into_string()`
//!
//! ## Serde
//!
//! [`Serialize`] and [`Deserialize`] implementations from the [`serde`] crate
//! can be automatically generated by including `serde` in the argument list for the macro.
//!
//!   [`serde`]: https://docs.rs/serde/*/serde/
//!   [`Serialize`]: https://docs.rs/serde/*/serde/trait.Serialize.html
//!   [`Deserialize`]: https://docs.rs/serde/*/serde/trait.Deserialize.html
//!
//! ```
//!# use braid::braid;
//!#
//! #[braid(serde)]
//! pub struct Username;
//!
//! let username = Username::new("root");
//! let json = serde_json::to_string(&username).unwrap();
//! let new_username: Username = serde_json::from_str(&json).unwrap();
//!# assert_eq!(username, new_username);
//! ```
//!
//! Such automatic implementations will also properly handle string values that require
//! validation. This automatic validation has the benefit of easing use with _Serde_ while
//! still protecting the integrity of the type.
//!
//! ```
//!# use braid::braid;
//!#
//! #[derive(Debug, PartialEq, Eq)]
//! pub struct InvalidUsername;
//! // Error implementation elided
//!# impl std::fmt::Display for InvalidUsername {
//!#     fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
//!#         f.write_str("invalid username")
//!#     }
//!# }
//!# impl std::error::Error for InvalidUsername {}
//!
//! #[braid(serde, validator)]
//! pub struct Username;
//!
//! impl braid::Validator for Username {
//!     type Error = InvalidUsername;
//!     fn validate(s: &str) -> Result<(), Self::Error> {
//!         if s.is_empty() || s.eq_ignore_ascii_case("root") {
//!             Err(InvalidUsername)
//!         } else {
//!             Ok(())
//!         }
//!     }
//! }
//!
//! assert!(serde_json::from_str::<Username>("\"\"").is_err());
//! assert!(serde_json::from_str::<Username>("\"root\"").is_err());
//! assert!(serde_json::from_str::<Username>("\"nobody\"").is_ok());
//!
//! assert!(serde_json::from_str::<&UsernameRef>("\"\"").is_err());
//! assert!(serde_json::from_str::<&UsernameRef>("\"root\"").is_err());
//! assert!(serde_json::from_str::<&UsernameRef>("\"nobody\"").is_ok());
//! ```
//!
//! ## Safety
//!
//! Braid uses limited `unsafe` in order to be able to reinterpret string slices
//! ([`&str`]) as the borrowed form. Because this functionality is provided as a
//! macro, using the `#![forbid(unsafe_code)]` lint level on a crate that generates
//! braids will result in compiler errors. Instead, the crate can be annotated with
//! `#![deny(unsafe_code)]`, which allows for overrides as appropriate. The functions
//! that require `unsafe` to work correctly are annotated with `#[allow(unsafe_code)]`.
//!
//! If strict adherence to forbid unsafe code is required, then the types can be
//! segregated into an accessory crate without the prohibition, and then consumed
//! safely from crates that otherwise forbid unsafe code.
//!

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

/// A validator that can verify a given is valid given certain preconditions
pub trait Validator {
    /// The error produced when the string is invalid
    type Error: std::error::Error + Send + Sync + 'static;

    /// Validates a string according to a predetermined set of rules
    fn validate(s: &str) -> Result<(), Self::Error>;
}

pub use braid_impl::braid;
pub use braid_impl::braid_ref;
