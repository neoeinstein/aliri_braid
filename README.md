# aliri_braid

[![Build Status](https://github.com/neoeinstein/aliri_braid/actions/workflows/rust.yml/badge.svg?branch=main&event=push)](https://github.com/neoeinstein/aliri_braid)

Improve and strengthen your strings

Strongly-typed APIs reduce errors and confusion over passing around un-typed strings.
Braid helps in that endeavor by making it painless to create wrappers around your
string values, ensuring that you use them in the right way every time.

## Usage

A braid is created by attaching `#[braid]` to a struct definition. The macro will take
care of automatically updating the representation of the struct to wrap a string and
generate the borrowed form of the strong type.

```rust
use aliri_braid::braid;

#[braid]
pub struct DatabaseName;
```

Once created, braids can be passed around as strongly-typed strings.

```rust
#
fn take_strong_string(n: DatabaseName) {}
fn borrow_strong_string(n: &DatabaseNameRef) {}

#
let owned = DatabaseName::new("mongo");
borrow_strong_string(&owned);
take_strong_string(owned);
```

A braid can also be untyped for use in stringly-typed interfaces.

```rust
#
fn take_raw_string(s: String) {}
fn borrow_raw_str(s: &str) {}

#
let owned = DatabaseName::new("mongo");
borrow_raw_str(owned.as_str());
take_raw_string(owned.into_string());
```

By default, the name of the borrowed form will be the same as the owned form
with `Ref` appended to the end.

```rust
#
#[braid]
pub struct DatabaseName;

let owned = DatabaseName::new("mongo");
let borrowed = DatabaseNameRef::from_str("mongo");
```

If the name ends with `Buf`, however, then the borrowed form will drop the `Buf`, similar
to the relationship between
[`PathBuf`][std::path::PathBuf] and [`Path`][std::path::Path].

```rust
#
#[braid]
pub struct DatabaseNameBuf;

let owned = DatabaseNameBuf::new("mongo");
let borrowed = DatabaseName::from_str("mongo");
```

If a different name is desired, this behavior can be
overridden by specifying the name of the reference type to create using the `ref`
parameter.

```rust
#
#[braid(ref = "TempDb")]
pub struct DatabaseNameBuf;

let owned = DatabaseNameBuf::new("mongo");
let borrowed = TempDb::from_str("mongo");
let to_owned: DatabaseNameBuf = borrowed.to_owned();
```

A default doc comment is added to the borrowed form that refers back to the owned form.
If a custom doc comment is desired, the `ref_doc` parameter allows supplying custom
documentation.

```rust
#
#[braid(ref_doc = "A temporary reference to a database name")]
pub struct DatabaseName;
#
```

## Extensibility

The types created by the `braid` macro are placed in the same module where declared.
This means additional functionality, including mutations, can be implemented easily.

As a basic example, here is a type built to hold Amazon ARNs. The type has been
extended to support some mutation and introspection.

```rust
#
#[braid]
pub struct AmazonArnBuf;

impl AmazonArnBuf {
    /// Append an ARN segment
    pub fn add_segment(&mut self, segment: &str) {
        self.0.push_str(":");
        self.0.push_str(segment);
    }
}

impl AmazonArn {
    /// Returns an iterator of all ARN segments
    pub fn get_segments(&self) -> std::str::Split<char> {
        self.0.split(':')
    }

    /// Returns the service segment of the ARN
    pub fn get_service(&self) -> &str {
        self.get_segments().nth(2).unwrap_or("")
    }
}
```

## Encapsulation

Because code within the same module where the braid is defined are allowed to
access the internal value, you can use a module in order to more strictly
enforce encapsulation and limit accessibility that might otherwise violate
established invariants. This may be particularly desired when the wrapped type
requires [validation](#validation).

```rust
mod amazon_arn {
    #[aliri_braid::braid]
    pub struct AmazonArnBuf;

    /* Additional impls that need access to the inner values */
#
}

pub use amazon_arn::{AmazonArnBuf, AmazonArn};

let x = AmazonArnBuf::new("arn:aws:iam::123456789012:user/Development");
assert_eq!("iam", x.get_service());
```

## Soundness

This crate ensures that the `from_str` implementation provided for wrapping
borrowed `str` slices does not extend lifetimes.

In the example below, we verify that the borrowed `DatabaseNameRef` is unable
to escape the lifetime of `data`. The following code snippet will fail to
compile, because `data` will go out of scope and be dropped at the end of
the block creating `ex_ref`.

```compile_fail
# use aliri_braid::braid;
#
# #[braid]
# pub struct DatabaseName;
#
let ex_ref = {
    let data = DatabaseName::new("test string");
    DatabaseNameRef::from_str(data.as_str())
}; // `data` is dropped at this point

// Which means that `ex_ref` would be invalid if allowed.
println!("{}", ex_ref);
```

## Validation

Types can be configured to only contain certain values. This can be used to strongly
enforce domain type boundaries, thus making invalid values unrepresentable.

For example, if you wanted to have a username type that did not accept the `root` user,
you have a few options:

1. Pass the username around as a string, validate that it isn't `root` at known entry points.
2. Create a username type and allow creation from a raw string, then validate it
   just after creation.
3. Create a strong username type that requires the value to be validated prior to being
   creatable.

Braided strings give the strongest, third guarantee. The other two methods require constant
vigilance to ensure that an unexpected `root` value doesn't sneak in through other backdoors.

By default, Rust's module system allows items within the same module to have access to
each other's non-public members. If not handled properly, this can lead to unintentionally
violating invariants. Thus, for the strongest guarantees, it is recommended to use the module
system to further control access to the interior values held by the braided type as
described in the section on [encapsulation](#encapsulation).

```rust
#
#[derive(Debug, PartialEq, Eq)]
pub struct InvalidUsername;
// Error implementation elided

#[braid(validator)]
pub struct NonRootUsername;

impl aliri_braid::Validator for NonRootUsername {
    type Error = InvalidUsername;
    fn validate(s: &str) -> Result<(), Self::Error> {
        if s.is_empty() || s.eq_ignore_ascii_case("root") {
            Err(InvalidUsername)
        } else {
            Ok(())
        }
    }
}

assert!(NonRootUsername::new("").is_err());
assert!(NonRootUsername::new("root").is_err());
assert!(NonRootUsername::new("nobody").is_ok());

assert!(NonRootUsernameRef::from_str("").is_err());
assert!(NonRootUsernameRef::from_str("root").is_err());
assert!(NonRootUsernameRef::from_str("nobody").is_ok());
```

Foreign validators can also be used by specifying the name of the type that
implements the validation logic.

```rust
#
#
#[braid(validator = "UsernameValidator")]
pub struct NonRootUsername;

pub struct UsernameValidator;

impl aliri_braid::Validator for UsernameValidator {
    /* … */
}

assert!(NonRootUsername::new("").is_err());
assert!(NonRootUsername::new("root").is_err());
assert!(NonRootUsername::new("nobody").is_ok());

assert!(NonRootUsernameRef::from_str("").is_err());
assert!(NonRootUsernameRef::from_str("root").is_err());
assert!(NonRootUsernameRef::from_str("nobody").is_ok());
```

### Normalization

Braided strings can also have enforced normalization, which is carried out at the creation
boundary. In this case, the `.from_str()` function on the borrowed form will return a
[`Cow`][std::borrow::Cow]`<Borrowed>`, which can be inspected to determine whether
normalization and conversion to an owned value was required. In cases where the incoming
value is expected to already be normalized, the `.from_normalized_str()` function can
be used. This function will return an error if the value required normalization.

When using `serde` to deserialze directly to the borrowed form, care must be taken, as
only already normalized values will be able to be deserialized. If normalization is
expected, deserialize into the owned form or `Cow<Borrowed>`.

Here is a toy example where the value must not be empty and must be composed of ASCII
characters, but that is also normalized to use lowercase ASCII letters.

```rust
use std::borrow::Cow;

#[derive(Debug, PartialEq, Eq)]
pub struct InvalidHeaderName;
// Error implementation elided

#[braid(normalizer)]
pub struct HeaderName;

impl aliri_braid::Normalizer for HeaderName {
    type Error = InvalidHeaderName;
    fn normalize(s: &str) -> Result<Cow<str>, Self::Error> {
        if !s.is_ascii() || s.is_empty() {
            Err(InvalidHeaderName)
        } else if s.as_bytes().iter().any(|&b| b'A' <= b && b <= b'Z') {
            Ok(Cow::Owned(s.to_ascii_lowercase()))
        } else {
            Ok(Cow::Borrowed(s))
        }
    }
}

assert!(HeaderName::new("").is_err());
assert_eq!("mixedcase", HeaderName::new("MixedCase").unwrap().as_str());
assert_eq!("lowercase", HeaderName::new("lowercase").unwrap().as_str());

assert!(HeaderNameRef::from_str("").is_err());
assert_eq!("mixedcase", HeaderNameRef::from_str("MixedCase").unwrap().as_str());
assert_eq!("lowercase", HeaderNameRef::from_str("lowercase").unwrap().as_str());

assert!(HeaderNameRef::from_normalized_str("").is_err());
assert!(HeaderNameRef::from_normalized_str("MixedCase").is_err());
assert_eq!("lowercase", HeaderNameRef::from_normalized_str("lowercase").unwrap().as_str());
```

### Unchecked creation

Where necessary for efficiency, it is possible to bypass the validations on creation through
the use of the `.new_unchecked()` or `from_str_unchecked()` functions. These functions are
marked as `unsafe`, as they require the caller to assert that they are fulfilling the
implicit contract that the value be both valid and in normal form. If either of these
constraints are violated, undefined behavior could result when downstream consumers depend
on these constraints being upheld.

```compile_fail
# use aliri_braid::braid;
#
# #[derive(Debug, PartialEq, Eq)]
# pub struct InvalidUsername;
# // Error implementation elided
# impl std::fmt::Display for InvalidUsername {
#     fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
#         f.write_str("invalid username")
#     }
# }
# impl std::error::Error for InvalidUsername {}
#
# #[braid(validator)]
# pub struct NonRootUsername;
#
# impl aliri_braid::Validator for NonRootUsername {
#     type Error = InvalidUsername;
#     fn validate(s: &str) -> Result<(), Self::Error> {
#         if s.is_empty() || s.eq_ignore_ascii_case("root") {
#             Err(InvalidUsername)
#         } else {
#             Ok(())
#         }
#     }
# }
#
NonRootUsername::new_unchecked("");
NonRootUsernameRef::from_str_unchecked("nobody");
```

If you find violations of your guarantees, you can look specifically for uses of `unsafe`.

```rust
#
#
#
#
unsafe {
    NonRootUsername::new_unchecked("");
    NonRootUsernameRef::from_str_unchecked("root");
}
```

## Provided trait impls

By default, the following traits will be automatically implemented.

For the `Owned` type
* [`std::clone::Clone`]
* [`std::fmt::Debug`]
* [`std::fmt::Display`]
* [`std::hash::Hash`]
* [`std::cmp::Eq`]
* [`std::cmp::PartialEq<Owned>`]
* [`std::cmp::PartialEq<Borrowed>`]
* [`std::cmp::PartialEq<&Borrowed>`]
* [`std::cmp::PartialEq<Box<Borrowed>>`]
* [`std::convert::AsRef<Borrowed>`]
* [`std::convert::AsRef<str>`]
* [`std::convert::From<&Borrowed>`]
* [`std::borrow::Borrow<Borrowed>`]
* [`std::str::FromStr`]
* [`std::ops::Deref`] where `Target = Borrowed`

Additionally, unvalidated owned types implement
* [`std::convert::From<String>`]
* [`std::convert::From<&str>`]

Validated and normalized owned types will instead implement
* [`std::convert::TryFrom<String>`]
* [`std::convert::TryFrom<&str>`]

When normalized, the above conversions will normalize values.

For the `Borrowed` type
* [`std::fmt::Debug`]
* [`std::fmt::Display`]
* [`std::hash::Hash`]
* [`std::cmp::Eq`]
* [`std::cmp::PartialEq<Owned>`]
* [`std::cmp::PartialEq<Borrowed>`]
* [`std::cmp::PartialEq<&Borrowed>`]
* [`std::cmp::PartialEq<Box<Borrowed>>`]
* [`std::borrow::ToOwned`] where `Owned = Owned`
* [`std::convert::AsRef<str>`]

Additionally, unvalidated borrowed types implement
* [`std::convert::From<&str>`]

Validated and normalize borrowed types will instead implement
* [`std::convert::TryFrom<&str>`]

The above conversion will fail if the value is not already normalized.

`Deref` to a `str` is explicitly not implemented. This means that an explicit call is
required to treat a value as an untyped string, whether `.as_str()`, `.to_string()`, or
`.into_string()`

## Serde

[`Serialize`] and [`Deserialize`] implementations from the [`serde`] crate
can be automatically generated by including `serde` in the argument list for the macro.

  [`serde`]: https://docs.rs/serde/*/serde/
  [`Serialize`]: https://docs.rs/serde/*/serde/trait.Serialize.html
  [`Deserialize`]: https://docs.rs/serde/*/serde/trait.Deserialize.html

```rust
#
#[braid(serde)]
pub struct Username;

let username = Username::new("root");
let json = serde_json::to_string(&username).unwrap();
let new_username: Username = serde_json::from_str(&json).unwrap();
```

Such automatic implementations will also properly handle string values that require
validation. This automatic validation has the benefit of easing use with _Serde_ while
still protecting the integrity of the type.

```rust
#
#[derive(Debug, PartialEq, Eq)]
pub struct InvalidUsername;
// Error implementation elided

#[braid(serde, validator)]
pub struct Username;

impl aliri_braid::Validator for Username {
    type Error = InvalidUsername;
    fn validate(s: &str) -> Result<(), Self::Error> {
        if s.is_empty() || s.eq_ignore_ascii_case("root") {
            Err(InvalidUsername)
        } else {
            Ok(())
        }
    }
}

assert!(serde_json::from_str::<Username>("\"\"").is_err());
assert!(serde_json::from_str::<Username>("\"root\"").is_err());
assert!(serde_json::from_str::<Username>("\"nobody\"").is_ok());

assert!(serde_json::from_str::<&UsernameRef>("\"\"").is_err());
assert!(serde_json::from_str::<&UsernameRef>("\"root\"").is_err());
assert!(serde_json::from_str::<&UsernameRef>("\"nobody\"").is_ok());
```

## Safety

Braid uses limited `unsafe` in order to be able to reinterpret string slices
([`&str`]) as the borrowed form. Because this functionality is provided as a
macro, using the `#![forbid(unsafe_code)]` lint level on a crate that generates
braids will result in compiler errors. Instead, the crate can be annotated with
`#![deny(unsafe_code)]`, which allows for overrides as appropriate. The functions
that require `unsafe` to work correctly are annotated with `#[allow(unsafe_code)]`,
and all usages of unsafe that the macro generates are annotated with `SAFETY`
code comments.

If strict adherence to forbid unsafe code is required, then the types can be
segregated into an accessory crate without the prohibition, and then consumed
safely from crates that otherwise forbid unsafe code.

