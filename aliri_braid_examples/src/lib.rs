//! Examples showing the output of using [`aliri_braid`] to
//! generate strongly-typed wrappers around string values.
//!
//! Three types of braids are demonstrated:
//! * [Wrapper][wrapper] (and an example with [small-string optimization][sso_wrapper])
//! * [Validated][validated]
//! * [Normalized][normalized]
//!
//! In addition, the `minimal` module demonstrates the minimal string
//! implementation that can be wrapped inside a braid type.
#![deny(unsafe_code)]

pub mod minimal;
pub mod normalized;
pub mod ref_only;
pub mod sso_wrapper;
pub mod validated;
pub mod wrapper;
