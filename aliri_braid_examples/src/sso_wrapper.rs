//! An example of constructing a basic strongly-typed wrapper around
//! a string value.
//!
//! The types in this module do not perform any validation or normalization
//! of their values, so every valid UTF-8 string is potentially valid for
//! these types.

use aliri_braid::braid;
use smartstring::alias::String;

/// An example of a wrapper around a [`smartstring::SmartString`] with
/// small-string optimization
///
/// This type ends in _Buf_, so the borrowed form of this type
/// will be named [`SmartUsername`].
///
/// Because the no type is explicitly named here, the inner field will
/// implicitly use the `String` type in the namespace where it is defined.
#[braid(serde, ref_doc = "A borrowed reference to a string slice wrapper")]
pub struct SmartUsernameBuf;

// This doesn't work right now as `CompactString` doesn't implement `Into<String>`.
//
// /// An example of a wrapper around a [`compact_str::CompactString`] with
// /// small-string optimization
// ///
// /// This type ends in _Buf_, so the borrowed form of this type
// /// will be named [`CompactUsername`].
// ///
// /// Because the no type is explicitly named here, the inner field will
// /// implicitly use the `String` type in the namespace where it is defined.
// #[braid(
//     serde,
//     ref_doc = "A borrowed reference to a string slice wrapper"
// )]
// pub struct CompactUsernameBuf(compact_str::CompactString);