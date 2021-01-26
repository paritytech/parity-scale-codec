# Changelog
All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this crate adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.0.0] - 2021-01-26

### Added
- `Decode::skip` is introduced, allowing to skip some encoded types.
- `Decode::encoded_fixed_size` is introduced, allowing to get the fixed encoded size of a type.
- `Error` now contains a chain of causes. This full error description can also be activated on
  no std using the feature `chain-error`.
- `Encode::encoded_size` is introduced, allowing to get the encoded size of a type more efficiently.

### Changed
- `CompactAs::decode_from` now returns result. This allow for decoding to fail from their compact
  form.
- derive macro use literal index e.g. `#[codec(index = 15)]` instead of `#[codec(index = "15")]`
- Version of crates `bitvec` and `generic-array` is updated.
- `Encode::encode_to` now bounds the generic `W: Output + ?Sized` instead of `W: Output`.
- `Output` can now be used as a trait object.

### Removed
- `EncodeAppend::append` is removed in favor of `EncodeAppend::append_or_new`.
- `Output::push` is removed in favor of `Encode::encode_to`.
- Some bounds on `HasCompact::Type` are removed.
- `Error::what` is removed in favor of `Error::to_string` (implemented through trait `Display`).
- `Error::description` is removed in favor of `Error::to_string` (implemented through trait `Display`).
