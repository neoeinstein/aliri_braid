# Changelog

The format of this changelog is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

### Added

- `from_infallible!()` utility macro added for providing universal `From<Infallible>` impls
- Added an example using `CompactString` from the `compact_str` crate ([#15])

### Changed

- `Validator::Error` is now expected to implement `From<Infallible>` to allow potentially
  fallible wrapped type conversions

[#15]: https://github.com/neoeinstein/aliri_braid/pull/15

## [0.2.3] - 2022-06-15

### Changed

- Added allow for clippy lint on automatically derived `serde::Deserialize` impls

<!--markdownlint-disable-file MD024 -->