# Changelog

All notable changes to this crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this crate adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.3.1] - 2021-09-28

### Fix

- Improve macro hygiene of `Encode` and `Decode` proc. macro expansions. ([#291](https://github.com/paritytech/parity-scale-codec/pull/291), [#293](https://github.com/paritytech/parity-scale-codec/pull/293))

## [2.3.0] - 2021-09-11

### Added
- `decode_and_advance_with_depth_limit` to the `DecodeLimit` trait. This allows advancing the cursor while decoding the input. PR #286

## [2.2.0] - 2021-07-02

### Added

- Add support for custom where bounds `codec(mel_bound(T: MaxEncodedLen))` when deriving the traits. PR #279
- `MaxEncodedLen` trait for items that have a statically known maximum encoded size. ([#268](https://github.com/paritytech/parity-scale-codec/pull/268))
- `#[codec(crate = <path>)]` top-level attribute to be used with the new `MaxEncodedLen`
trait, which allows to specify a different path to the crate that contains the `MaxEncodedLen` trait.
Useful when using generating a type through a macro and this type should implement `MaxEncodedLen` and the final crate doesn't have `parity-scale-codec` as dependency.

## [2.1.3] - 2021-06-14

### Changed

- Lint attributes now pass through to the derived impls of `Encode`, `Decode` and `CompactAs`. PR #272

## [2.1.0] - 2021-04-06

### Fix

- Add support for custom where bounds `codec(encode_bound(T: Encode))` and `codec(decode_bound(T: Decode))` when
deriving the traits. Pr #262
- Switch to const generics for array implementations. Pr #261

## [2.0.1] - 2021-02-26

### Fix

- Fix type inference issue in `Decode` derive macro. Pr #254

## [2.0.0] - 2021-01-26

### Added

- `Decode::skip` allows to skip some encoded types. Pr #243
- `Decode::encoded_fixed_size` allows to get the fixed encoded size of a type. PR #243
- `Error` now contains a chain of causes. This full error description can also be activated on
  no std using the feature `chain-error`. PR #242
- `Encode::encoded_size` allows to get the encoded size of a type more efficiently. PR #245

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
