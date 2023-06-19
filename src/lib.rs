// Copyright 2017-2021 Parity Technologies
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![doc = include_str!("../README.md")]
#![warn(missing_docs)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
#[macro_use]
#[doc(hidden)]
pub extern crate alloc;

#[cfg(feature = "derive")]
#[allow(unused_imports)]
#[macro_use]
extern crate parity_scale_codec_derive;

#[cfg(all(feature = "std", test))]
#[macro_use]
extern crate serde_derive;

#[cfg(feature = "derive")]
pub use parity_scale_codec_derive::*;

#[cfg(feature = "std")]
#[doc(hidden)]
pub mod alloc {
	pub use std::boxed;
	pub use std::vec;
	pub use std::string;
	pub use std::borrow;
	pub use std::collections;
	pub use std::sync;
	pub use std::rc;
	pub use std::alloc;
}

mod codec;
mod compact;
mod joiner;
mod keyedvec;
#[cfg(feature = "bit-vec")]
mod bit_vec;
#[cfg(feature = "generic-array")]
mod generic_array;
mod decode_all;
mod decode_finished;
mod depth_limit;
mod encode_append;
mod encode_like;
mod error;
#[cfg(feature = "max-encoded-len")]
mod max_encoded_len;
#[cfg(feature = "max-encoded-len")]
mod const_encoded_len;

pub use self::error::Error;
pub use self::codec::{
	Input, Output, Decode, Encode, Codec, EncodeAsRef, WrapperTypeEncode, WrapperTypeDecode,
	OptionBool, DecodeLength, FullCodec, FullEncode, decode_vec_with_len,
};
#[cfg(feature = "std")]
pub use self::codec::IoReader;
pub use self::compact::{Compact, HasCompact, CompactAs, CompactLen, CompactRef};
pub use self::joiner::Joiner;
pub use self::keyedvec::KeyedVec;
pub use self::decode_all::DecodeAll;
pub use self::decode_finished::DecodeFinished;
pub use self::depth_limit::DecodeLimit;
pub use self::encode_append::EncodeAppend;
pub use self::encode_like::{EncodeLike, Ref};
#[cfg(feature = "max-encoded-len")]
pub use max_encoded_len::MaxEncodedLen;
#[cfg(feature = "max-encoded-len")]
pub use const_encoded_len::ConstEncodedLen;

/// Derive macro for [`MaxEncodedLen`][max_encoded_len::MaxEncodedLen].
///
/// # Examples
///
/// ```
/// # use parity_scale_codec::{Encode, MaxEncodedLen};
/// #[derive(Encode, MaxEncodedLen)]
/// struct Example;
/// ```
///
/// ```
/// # use parity_scale_codec::{Encode, MaxEncodedLen};
/// #[derive(Encode, MaxEncodedLen)]
/// struct TupleStruct(u8, u32);
///
/// assert_eq!(TupleStruct::max_encoded_len(), u8::max_encoded_len() + u32::max_encoded_len());
/// ```
///
/// ```
/// # use parity_scale_codec::{Encode, MaxEncodedLen};
/// #[derive(Encode, MaxEncodedLen)]
/// enum GenericEnum<T> {
///     A,
///     B(T),
/// }
///
/// assert_eq!(GenericEnum::<u8>::max_encoded_len(), u8::max_encoded_len() + u8::max_encoded_len());
/// assert_eq!(GenericEnum::<u128>::max_encoded_len(), u8::max_encoded_len() + u128::max_encoded_len());
/// ```
///
/// # Within other macros
///
/// Sometimes the `MaxEncodedLen` trait and macro are used within another macro, and it can't be
/// guaranteed that the `parity_scale_codec` module is available at the call site. In that case, the
/// macro should reexport the `parity_scale_codec` module and specify the path to the reexport:
///
/// ```ignore
/// pub use parity_scale_codec as codec;
///
/// #[derive(Encode, MaxEncodedLen)]
/// #[codec(crate = $crate::codec)]
/// struct Example;
/// ```
#[cfg(all(feature = "derive", feature = "max-encoded-len"))]
pub use parity_scale_codec_derive::MaxEncodedLen;

#[cfg(feature = "bytes")]
pub use self::codec::decode_from_bytes;
