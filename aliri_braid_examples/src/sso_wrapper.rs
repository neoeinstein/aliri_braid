//! An example of constructing a strongly-typed wrapper around
//! a string with small-string optimization.
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

/// An example of a wrapper with small-string optimization
///
/// This type wraps the around a [`compact_str::CompactString`], but that
/// implementation detail won't be exposed through the type API due to
/// the use of the `no_expose` braid parameter.
#[braid(serde, no_expose)]
pub struct CompactData(compact_str::CompactString);
