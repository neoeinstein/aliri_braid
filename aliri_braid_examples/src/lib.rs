//! Examples showing the output of using [`aliri_braid`] to
//! generate strongly-typed wrappers around string values.
//!
//! Three types of braids are demonstrated:
//! * [Wrapper][wrapper]
//! * [Validated][validated]
//! * [Normalized][normalized]
#![deny(unsafe_code)]

pub mod normalized;
pub mod validated;
pub mod wrapper;
