// Copyright 2017, 2018 Parity Technologies
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

//! Serialisation.

use crate::alloc::vec::Vec;
use crate::alloc::boxed::Box;
use crate::alloc::collections::btree_map::BTreeMap;

#[cfg(any(feature = "std", feature = "full"))]
use crate::alloc::{
	string::String,
	borrow::Cow,
};

use core::{mem, slice, ops::Deref};
use arrayvec::ArrayVec;
use core::marker::PhantomData;

#[cfg(feature = "std")]
use std::fmt;

#[cfg_attr(feature = "std", derive(Debug))]
#[derive(PartialEq)]
#[cfg(feature = "std")]
/// Descriptive error type
pub struct Error(&'static str);

#[cfg(not(feature = "std"))]
#[derive(PartialEq)]
pub struct Error;

impl Error {
	#[cfg(feature = "std")]
	/// Error description
	///
	/// This function returns an actual error str when running in `std`
	/// environment, but `""` on `no_std`.
	pub fn what(&self) -> &'static str {
		self.0
	}

	#[cfg(not(feature = "std"))]
	/// Error description
	///
	/// This function returns an actual error str when running in `std`
	/// environment, but `""` on `no_std`.
	pub fn what(&self) -> &'static str {
		""
	}
}

#[cfg(feature = "std")]
impl std::fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {
	fn description(&self) -> &str {
		self.0
	}
}

impl From<&'static str> for Error {
	#[cfg(feature = "std")]
	fn from(s: &'static str) -> Error {
		return Error(s)
	}

	#[cfg(not(feature = "std"))]
	fn from(_s: &'static str) -> Error {
		return Error
	}
}

/// Trait that allows reading of data into a slice.
pub trait Input {
	/// Read into the provided input slice. Returns the number of bytes read.
	///
	/// Note that this function should be more like `std::io::Read::read_exact`
	/// than `std::io::Read::read`. I.e. the buffer should always be filled
	/// with as many bytes as available and if `n < into.len()` is returned
	/// then it should mean that there was not enough bytes available and the
	/// `Input` is drained.
	///
	/// Callers of this function should not need to call again if `n < into.len()`
	/// is returned.
	fn read(&mut self, into: &mut [u8]) -> Result<usize, Error>;

	/// Read a single byte from the input.
	fn read_byte(&mut self) -> Result<u8, Error> {
		let mut buf = [0u8];
		self.read(&mut buf[..])?;
		Ok(buf[0])
	}
}

#[cfg(not(feature = "std"))]
impl<'a> Input for &'a [u8] {
	fn read(&mut self, into: &mut [u8]) -> Result<usize, Error> {
		if into.len() > self.len() {
			return Err("".into());
		}
		let len = ::core::cmp::min(into.len(), self.len());
		into[..len].copy_from_slice(&self[..len]);
		*self = &self[len..];
		Ok(len)
	}
}

#[cfg(feature = "std")]
impl From<std::io::Error> for Error {
	fn from(_err: std::io::Error) -> Self {
		"io error".into()
	}
}

#[cfg(feature = "std")]
impl<R: std::io::Read> Input for R {
	fn read(&mut self, into: &mut [u8]) -> Result<usize, Error> {
		(self as &mut dyn std::io::Read).read_exact(into)?;
		Ok(into.len())
	}
}

/// Prefix another input with a byte.
struct PrefixInput<'a, T> {
	prefix: Option<u8>,
	input: &'a mut T,
}

impl<'a, T: 'a + Input> Input for PrefixInput<'a, T> {
	fn read(&mut self, buffer: &mut [u8]) -> Result<usize, Error> {
		match self.prefix.take() {
			Some(v) if buffer.len() > 0 => {
				buffer[0] = v;
				let res = 1 + self.input.read(&mut buffer[1..])?;
				Ok(res)
			}
			_ => self.input.read(buffer)
		}
	}
}

/// Trait that allows writing of data.
pub trait Output: Sized {
	/// Write to the output.
	fn write(&mut self, bytes: &[u8]);

	/// Write a single byte to the output.
	fn push_byte(&mut self, byte: u8) {
		self.write(&[byte]);
	}

	/// Write encoding of given value to the output.
	fn push<V: Encode + ?Sized>(&mut self, value: &V) {
		value.encode_to(self);
	}
}

#[cfg(not(feature = "std"))]
impl Output for Vec<u8> {
	fn write(&mut self, bytes: &[u8]) {
		self.extend_from_slice(bytes)
	}
}

#[cfg(feature = "std")]
impl<W: std::io::Write> Output for W {
	fn write(&mut self, bytes: &[u8]) {
		(self as &mut dyn std::io::Write).write_all(bytes).expect("Codec outputs are infallible");
	}
}

struct ArrayVecWrapper<T: arrayvec::Array>(ArrayVec<T>);

impl<T: arrayvec::Array<Item=u8>> Output for ArrayVecWrapper<T> {
	fn write(&mut self, bytes: &[u8]) {
		self.0.extend(bytes.iter().map(|i| *i));
	}

	fn push_byte(&mut self, byte: u8) {
		self.0.push(byte);
	}
}

/// Trait that allows zero-copy write of value-references to slices in LE format.
///
/// Implementations should override `using_encoded` for value types and `encode_to` and `size_hint` for allocating types.
/// Wrapper types should override all methods.
pub trait Encode {
	/// If possible give a hint of expected size of the encoding.
	///
	/// This method is used inside default implementation of `encode`
	/// to avoid re-allocations.
	fn size_hint(&self) -> usize {
		0
	}

	/// Convert self to a slice and append it to the destination.
	fn encode_to<T: Output>(&self, dest: &mut T) {
		self.using_encoded(|buf| dest.write(buf));
	}

	/// Convert self to an owned vector.
	fn encode(&self) -> Vec<u8> {
		let mut r = Vec::with_capacity(self.size_hint());
		self.encode_to(&mut r);
		r
	}

	/// Convert self to a slice and then invoke the given closure with it.
	fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
		f(&self.encode())
	}
}

/// Trait that allows zero-copy read of value-references from slices in LE format.
pub trait Decode: Sized {
	/// Attempt to deserialise the value from input.
	fn decode<I: Input>(value: &mut I) -> Result<Self, Error>;
}

/// Trait that allows zero-copy read/write of value-references to/from slices in LE format.
pub trait Codec: Decode + Encode {}
impl<S: Decode + Encode> Codec for S {}

/// A marker trait for types that wrap other encodable type.
///
/// Such types should not carry any additional information
/// that would require to be encoded, because the encoding
/// is assumed to be the same as the wrapped type.
pub trait WrapperTypeEncode: Deref {}

impl<T> WrapperTypeEncode for Vec<T> {}
impl<T: ?Sized> WrapperTypeEncode for Box<T> {}
impl<'a, T: ?Sized> WrapperTypeEncode for &'a T {}
impl<'a, T: ?Sized> WrapperTypeEncode for &'a mut T {}

#[cfg(any(feature = "std", feature = "full"))]
impl<'a, T: ToOwned + ?Sized> WrapperTypeEncode for Cow<'a, T> {}
#[cfg(any(feature = "std", feature = "full"))]
impl<T: ?Sized> WrapperTypeEncode for std::sync::Arc<T> {}
#[cfg(any(feature = "std", feature = "full"))]
impl<T: ?Sized> WrapperTypeEncode for std::rc::Rc<T> {}
#[cfg(any(feature = "std", feature = "full"))]
impl WrapperTypeEncode for String {}

impl<T, X> Encode for X where
	T: Encode + ?Sized,
	X: WrapperTypeEncode<Target=T>,
{
	fn size_hint(&self) -> usize {
		(&**self).size_hint()
	}

	fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
		(&**self).using_encoded(f)
	}

	fn encode(&self) -> Vec<u8> {
		(&**self).encode()
	}

	fn encode_to<W: Output>(&self, dest: &mut W) {
		(&**self).encode_to(dest)
	}
}

/// A marker trait for types that can be created solely from other decodable types.
///
/// The decoding of such type is assumed to be the same as the wrapped type.
pub trait WrapperTypeDecode: Sized {
	/// A wrapped type.
	type Wrapped: Into<Self>;
}
impl<T> WrapperTypeDecode for Box<T> {
	type Wrapped = T;
}
#[cfg(any(feature = "std", feature = "full"))]
impl<T> WrapperTypeDecode for std::sync::Arc<T> {
	type Wrapped = T;
}
#[cfg(any(feature = "std", feature = "full"))]
impl<T> WrapperTypeDecode for std::rc::Rc<T> {
	type Wrapped = T;
}

impl<T, X> Decode for X where
	T: Decode + Into<X>,
	X: WrapperTypeDecode<Wrapped=T>,
{
	fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
		Ok(T::decode(input)?.into())
	}
}

/// Compact-encoded variant of T. This is more space-efficient but less compute-efficient.
#[derive(Eq, PartialEq, Clone, Copy, Ord, PartialOrd)]
pub struct Compact<T>(pub T);

impl<T> From<T> for Compact<T> {
	fn from(x: T) -> Compact<T> { Compact(x) }
}

impl<'a, T: Copy> From<&'a T> for Compact<T> {
	fn from(x: &'a T) -> Compact<T> { Compact(*x) }
}

/// Allow foreign structs to be wrap in Compact
pub trait CompactAs: From<Compact<Self>> {
	/// A compact-encodable type that should be used as the encoding.
	type As;

	/// Returns the encodable type.
	fn encode_as(&self) -> &Self::As;

	/// Create `Self` from the decodable type.
	fn decode_from(_: Self::As) -> Self;
}

impl<T> Encode for Compact<T>
where
	for<'a> CompactRef<'a, T>: Encode,
{
	fn size_hint(&self) -> usize {
		CompactRef(&self.0).size_hint()
	}

	fn encode_to<W: Output>(&self, dest: &mut W) {
		CompactRef(&self.0).encode_to(dest)
	}

	fn encode(&self) -> Vec<u8> {
		CompactRef(&self.0).encode()
	}

	fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
		CompactRef(&self.0).using_encoded(f)
	}
}

impl<'a, T> Encode for CompactRef<'a, T>
where
	T: CompactAs,
	for<'b> CompactRef<'b, T::As>: Encode,
{
	fn size_hint(&self) -> usize {
		CompactRef(self.0.encode_as()).size_hint()
	}

	fn encode_to<Out: Output>(&self, dest: &mut Out) {
		CompactRef(self.0.encode_as()).encode_to(dest)
	}

	fn encode(&self) -> Vec<u8> {
		CompactRef(self.0.encode_as()).encode()
	}

	fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
		CompactRef(self.0.encode_as()).using_encoded(f)
	}
}

impl<T> Decode for Compact<T>
where
	T: CompactAs,
	Compact<T::As>: Decode,
{
	fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
		Compact::<T::As>::decode(input)
			.map(|x| Compact(<T as CompactAs>::decode_from(x.0)))
	}
}

macro_rules! impl_from_compact {
	( $( $ty:ty ),* ) => {
		$(
			impl From<Compact<$ty>> for $ty {
				fn from(x: Compact<$ty>) -> $ty { x.0 }
			}
		)*
	}
}

impl_from_compact! { (), u8, u16, u32, u64, u128 }

/// Compact-encoded variant of &'a T. This is more space-efficient but less compute-efficient.
#[derive(Eq, PartialEq, Clone, Copy)]
pub struct CompactRef<'a, T>(pub &'a T);

impl<'a, T> From<&'a T> for CompactRef<'a, T> {
	fn from(x: &'a T) -> Self { CompactRef(x) }
}

impl<T> ::core::fmt::Debug for Compact<T> where T: ::core::fmt::Debug {
	fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
		self.0.fmt(f)
	}
}

#[cfg(feature = "std")]
impl<T> serde::Serialize for Compact<T> where T: serde::Serialize {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
		T::serialize(&self.0, serializer)
	}
}

#[cfg(feature = "std")]
impl<'de, T> serde::Deserialize<'de> for Compact<T> where T: serde::Deserialize<'de> {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
		T::deserialize(deserializer).map(Compact)
	}
}

#[cfg(feature = "std")]
pub trait MaybeDebugSerde: core::fmt::Debug + serde::Serialize + for<'a> serde::Deserialize<'a> {}
#[cfg(feature = "std")]
impl<T> MaybeDebugSerde for T where T: core::fmt::Debug + serde::Serialize + for<'a> serde::Deserialize<'a> {}

#[cfg(not(feature = "std"))]
pub trait MaybeDebugSerde {}
#[cfg(not(feature = "std"))]
impl<T> MaybeDebugSerde for T {}

/// Trait that tells you if a given type can be encoded/decoded in a compact way.
pub trait HasCompact: Sized {
	/// The compact type; this can be
	type Type: for<'a> EncodeAsRef<'a, Self> + Decode + From<Self> + Into<Self> + Clone +
		PartialEq + Eq + MaybeDebugSerde;
}

/// Something that can be encoded as a reference.
pub trait EncodeAsRef<'a, T: 'a> {
	/// The reference type that is used for encoding.
	type RefType: Encode + From<&'a T>;
}

impl<'a, T: 'a> EncodeAsRef<'a, T> for Compact<T> where CompactRef<'a, T>: Encode + From<&'a T> {
	type RefType = CompactRef<'a, T>;
}

impl<T: 'static> HasCompact for T where
	Compact<T>: for<'a> EncodeAsRef<'a, T> + Decode + From<Self> + Into<Self> + Clone +
		PartialEq + Eq + MaybeDebugSerde,
{
	type Type = Compact<T>;
}

// compact encoding:
// 0b00 00 00 00 / 00 00 00 00 / 00 00 00 00 / 00 00 00 00
//   xx xx xx 00															(0 .. 2**6)		(u8)
//   yL yL yL 01 / yH yH yH yL												(2**6 .. 2**14)	(u8, u16)  low LH high
//   zL zL zL 10 / zM zM zM zL / zM zM zM zM / zH zH zH zM					(2**14 .. 2**30)	(u16, u32)  low LMMH high
//   nn nn nn 11 [ / zz zz zz zz ]{4 + n}									(2**30 .. 2**536)	(u32, u64, u128, U256, U512, U520) straight LE-encoded

// Note: we use *LOW BITS* of the LSB in LE encoding to encode the 2 bit key.

impl<'a> Encode for CompactRef<'a, ()> {
	fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
		f(&[])
	}
}

impl<'a> Encode for CompactRef<'a, u8> {
	fn encode_to<W: Output>(&self, dest: &mut W) {
		match self.0 {
			0..=0b00111111 => dest.push_byte(self.0 << 2),
			_ => (((*self.0 as u16) << 2) | 0b01).encode_to(dest),
		}
	}

	fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
		let mut r = ArrayVecWrapper(ArrayVec::<[u8; 2]>::new());
		self.encode_to(&mut r);
		f(&r.0)
	}
}

impl<'a> Encode for CompactRef<'a, u16> {
	fn encode_to<W: Output>(&self, dest: &mut W) {
		match self.0 {
			0..=0b00111111 => dest.push_byte((*self.0 as u8) << 2),
			0..=0b00111111_11111111 => ((*self.0 << 2) | 0b01).encode_to(dest),
			_ => (((*self.0 as u32) << 2) | 0b10).encode_to(dest),
		}
	}

	fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
		let mut r = ArrayVecWrapper(ArrayVec::<[u8; 4]>::new());
		self.encode_to(&mut r);
		f(&r.0)
	}
}

impl<'a> Encode for CompactRef<'a, u32> {
	fn encode_to<W: Output>(&self, dest: &mut W) {
		match self.0 {
			0..=0b00111111 => dest.push_byte((*self.0 as u8) << 2),
			0..=0b00111111_11111111 => (((*self.0 as u16) << 2) | 0b01).encode_to(dest),
			0..=0b00111111_11111111_11111111_11111111 => ((*self.0 << 2) | 0b10).encode_to(dest),
			_ => {
				dest.push_byte(0b11);
				self.0.encode_to(dest);
			}
		}
	}

	fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
		let mut r = ArrayVecWrapper(ArrayVec::<[u8; 5]>::new());
		self.encode_to(&mut r);
		f(&r.0)
	}
}

impl<'a> Encode for CompactRef<'a, u64> {
	fn encode_to<W: Output>(&self, dest: &mut W) {
		match self.0 {
			0..=0b00111111 => dest.push_byte((*self.0 as u8) << 2),
			0..=0b00111111_11111111 => (((*self.0 as u16) << 2) | 0b01).encode_to(dest),
			0..=0b00111111_11111111_11111111_11111111 => (((*self.0 as u32) << 2) | 0b10).encode_to(dest),
			_ => {
				let bytes_needed = 8 - self.0.leading_zeros() / 8;
				assert!(bytes_needed >= 4, "Previous match arm matches anyting less than 2^30; qed");
				dest.push_byte(0b11 + ((bytes_needed - 4) << 2) as u8);
				let mut v = *self.0;
				for _ in 0..bytes_needed {
					dest.push_byte(v as u8);
					v >>= 8;
				}
				assert_eq!(v, 0, "shifted sufficient bits right to lead only leading zeros; qed")
			}
		}
	}

	fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
		let mut r = ArrayVecWrapper(ArrayVec::<[u8; 9]>::new());
		self.encode_to(&mut r);
		f(&r.0)
	}
}

impl<'a> Encode for CompactRef<'a, u128> {
	fn encode_to<W: Output>(&self, dest: &mut W) {
		match self.0 {
			0..=0b00111111 => dest.push_byte((*self.0 as u8) << 2),
			0..=0b00111111_11111111 => (((*self.0 as u16) << 2) | 0b01).encode_to(dest),
			0..=0b00111111_11111111_11111111_11111111 => (((*self.0 as u32) << 2) | 0b10).encode_to(dest),
			_ => {
				let bytes_needed = 16 - self.0.leading_zeros() / 8;
				assert!(bytes_needed >= 4, "Previous match arm matches anyting less than 2^30; qed");
				dest.push_byte(0b11 + ((bytes_needed - 4) << 2) as u8);
				let mut v = *self.0;
				for _ in 0..bytes_needed {
					dest.push_byte(v as u8);
					v >>= 8;
				}
				assert_eq!(v, 0, "shifted sufficient bits right to lead only leading zeros; qed")
			}
		}
	}

	fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
		let mut r = ArrayVecWrapper(ArrayVec::<[u8; 17]>::new());
		self.encode_to(&mut r);
		f(&r.0)
	}
}

impl Decode for Compact<()> {
	fn decode<I: Input>(_input: &mut I) -> Result<Self, Error> {
		Ok(Compact(()))
	}
}

impl Decode for Compact<u8> {
	fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
		let prefix = input.read_byte()?;
		Ok(Compact(match prefix % 4 {
			0 => prefix as u8 >> 2,
			1 => {
				let x = u16::decode(&mut PrefixInput{prefix: Some(prefix), input})? >> 2;
				if x < 256 {
					x as u8
				} else {
					return Err("out of range decoding Compact<u8>".into());
				}
			}
			_ => return Err("unexpected prefix decoding Compact<u8>".into()),
		}))
	}
}

impl Decode for Compact<u16> {
	fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
		let prefix = input.read_byte()?;
		Ok(Compact(match prefix % 4 {
			0 => prefix as u16 >> 2,
			1 => u16::decode(&mut PrefixInput{prefix: Some(prefix), input})? as u16 >> 2,
			2 => {
				let x = u32::decode(&mut PrefixInput{prefix: Some(prefix), input})? >> 2;
				if x < 65536 {
					x as u16
				} else {
					return Err("out of range decoding Compact<u16>".into());
				}
			}
			_ => return Err("unexpected prefix decoding Compact<u16>".into()),
		}))
	}
}

impl Decode for Compact<u32> {
	fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
		let prefix = input.read_byte()?;
		Ok(Compact(match prefix % 4 {
			0 => prefix as u32 >> 2,
			1 => u16::decode(&mut PrefixInput{prefix: Some(prefix), input})? as u32 >> 2,
			2 => u32::decode(&mut PrefixInput{prefix: Some(prefix), input})? as u32 >> 2,
			3|_ => {	// |_. yeah, i know.
				if prefix >> 2 == 0 {
					// just 4 bytes. ok.
					u32::decode(input)?
				} else {
					// Out of range for a 32-bit quantity.
					return Err("out of range decoding Compact<u32>".into());
				}
			}
		}))
	}
}

impl Decode for Compact<u64> {
	fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
		let prefix = input.read_byte()?;
		Ok(Compact(match prefix % 4 {
			0 => prefix as u64 >> 2,
			1 => u16::decode(&mut PrefixInput{prefix: Some(prefix), input})? as u64 >> 2,
			2 => u32::decode(&mut PrefixInput{prefix: Some(prefix), input})? as u64 >> 2,
			3|_ => match (prefix >> 2) + 4 {
				4 => u32::decode(input)? as u64,
				8 => u64::decode(input)?,
				x if x > 8 => return Err("unexpected prefix decoding Compact<u64>".into()),
				bytes_needed => {
					let mut res = 0;
					for i in 0..bytes_needed {
						res |= (input.read_byte()? as u64) << (i * 8);
					}
					res
				}
			},
		}))
	}
}

impl Decode for Compact<u128> {
	fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
		let prefix = input.read_byte()?;
		Ok(Compact(match prefix % 4 {
			0 => prefix as u128 >> 2,
			1 => u16::decode(&mut PrefixInput{prefix: Some(prefix), input})? as u128 >> 2,
			2 => u32::decode(&mut PrefixInput{prefix: Some(prefix), input})? as u128 >> 2,
			3|_ => match (prefix >> 2) + 4 {
				4 => u32::decode(input)? as u128,
				8 => u64::decode(input)? as u128,
				16 => u128::decode(input)?,
				x if x > 16 => return Err("unexpected prefix decoding Compact<u128>".into()),
				bytes_needed => {
					let mut res = 0;
					for i in 0..bytes_needed {
						res |= (input.read_byte()? as u128) << (i * 8);
					}
					res
				}
			}
		}))
	}
}

impl<T: Encode, E: Encode> Encode for Result<T, E> {
	fn size_hint(&self) -> usize {
		1 + match *self {
			Ok(ref t) => t.size_hint(),
			Err(ref t) => t.size_hint(),
		}
	}

	fn encode_to<W: Output>(&self, dest: &mut W) {
		match *self {
			Ok(ref t) => {
				dest.push_byte(0);
				t.encode_to(dest);
			}
			Err(ref e) => {
				dest.push_byte(1);
				e.encode_to(dest);
			}
		}
	}
}

impl<T: Decode, E: Decode> Decode for Result<T, E> {
	fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
		match input.read_byte()? {
			0 => Ok(Ok(T::decode(input)?)),
			1 => Ok(Err(E::decode(input)?)),
			_ => Err("unexpected first byte decoding Result".into()),
		}
	}
}

/// Shim type because we can't do a specialised implementation for `Option<bool>` directly.
#[derive(Eq, PartialEq, Clone, Copy)]
pub struct OptionBool(pub Option<bool>);

impl core::fmt::Debug for OptionBool {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		self.0.fmt(f)
	}
}

impl Encode for OptionBool {
	fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
		f(&[match *self {
			OptionBool(None) => 0u8,
			OptionBool(Some(true)) => 1u8,
			OptionBool(Some(false)) => 2u8,
		}])
	}
}

impl Decode for OptionBool {
	fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
		match input.read_byte()? {
			0 => Ok(OptionBool(None)),
			1 => Ok(OptionBool(Some(true))),
			2 => Ok(OptionBool(Some(false))),
			_ => Err("unexpected first byte decoding OptionBool".into()),
		}
	}
}

impl<T: Encode> Encode for Option<T> {
	fn size_hint(&self) -> usize {
		1 + match *self {
			Some(ref t) => t.size_hint(),
			None => 0,
		}
	}

	fn encode_to<W: Output>(&self, dest: &mut W) {
		match *self {
			Some(ref t) => {
				dest.push_byte(1);
				t.encode_to(dest);
			}
			None => dest.push_byte(0),
		}
	}
}

impl<T: Decode> Decode for Option<T> {
	fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
		match input.read_byte()? {
			0 => Ok(None),
			1 => Ok(Some(T::decode(input)?)),
			_ => Err("unexpecded first byte decoding Option".into()),
		}
	}
}

macro_rules! impl_array {
	( $( $n:expr, )* ) => { $(
		impl<T: Encode> Encode for [T; $n] {
			fn encode_to<W: Output>(&self, dest: &mut W) {
				for item in self.iter() {
					item.encode_to(dest);
				}
			}
		}

		impl<T: Decode> Decode for [T; $n] {
			fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
				let mut r = ArrayVec::new();
				for _ in 0..$n {
					r.push(T::decode(input)?);
				}
				let i = r.into_inner();

				match i {
					Ok(a) => Ok(a),
					Err(_) => Err("failed to get inner array from ArrayVec".into()),
				}
			}
		}
		)* }
}

impl_array!(
	1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
	17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
	32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51,
	52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71,
	72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91,
	92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108,
	109, 110, 111, 112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123, 124,
	125, 126, 127, 128,	129, 130, 131, 132, 133, 134, 135, 136, 137, 138, 139, 140,
	141, 142, 143, 144, 145, 146, 147, 148, 149, 150, 151, 152, 153, 154, 155, 156,
	157, 158, 159, 160, 161, 162, 163, 164, 165, 166, 167, 168, 169, 170, 171, 172,
	173, 174, 175, 176, 177, 178, 179, 180, 181, 182, 183, 184, 185, 186, 187, 188,
	189, 190, 191, 192, 193, 194, 195, 196, 197, 198, 199, 200, 201, 202, 203, 204,
	205, 206, 207, 208, 209, 210, 211, 212, 213, 214, 215, 216, 217, 218, 219, 220,
	221, 222, 223, 224, 225, 226, 227, 228, 229, 230, 231, 232, 233, 234, 235, 236,
	237, 238, 239, 240, 241, 242, 243, 244, 245, 246, 247, 248, 249, 250, 251, 252,
	253, 254, 255, 256, 384, 512, 768, 1024, 2048, 4096, 8192, 16384, 32768,
);

impl Encode for [u8] {
	fn size_hint(&self) -> usize {
		self.len() + mem::size_of::<u32>()
	}

	fn encode_to<W: Output>(&self, dest: &mut W) {
		let len = self.len();
		assert!(len <= u32::max_value() as usize, "Attempted to serialize a collection with too many elements.");
		Compact(len as u32).encode_to(dest);
		dest.write(self)
	}
}

impl Decode for Vec<u8> {
	fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
		<Compact<u32>>::decode(input).and_then(move |Compact(len)| {
			let len = len as usize;
			let mut vec = vec![0; len];
			input.read(&mut vec[..len])?;
			Ok(vec)
		})
	}
}

impl Encode for str {
	fn size_hint(&self) -> usize {
		self.as_bytes().size_hint()
	}

	fn encode_to<W: Output>(&self, dest: &mut W) {
		self.as_bytes().encode_to(dest)
	}

	fn encode(&self) -> Vec<u8> {
		self.as_bytes().encode()
	}

	fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
		self.as_bytes().using_encoded(f)
	}
}

#[cfg(any(feature = "std", feature = "full"))]
impl<'a, T: ToOwned + ?Sized> Decode for Cow<'a, T>
where
<T as ToOwned>::Owned: Decode,
{
	fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
		Ok(Cow::Owned(Decode::decode(input)?))
	}
}

impl<T> Encode for PhantomData<T> {
	fn encode_to<W: Output>(&self, _dest: &mut W) {}
}

impl<T> Decode for PhantomData<T> {
	fn decode<I: Input>(_input: &mut I) -> Result<Self, Error> {
		Ok(PhantomData)
	}
}

#[cfg(any(feature = "std", feature = "full"))]
impl Decode for String {
	fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
		Ok(Self::from_utf8_lossy(&Vec::decode(input)?).into())
	}
}

impl<T: Encode> Encode for [T] {
	fn encode_to<W: Output>(&self, dest: &mut W) {
		let len = self.len();
		assert!(len <= u32::max_value() as usize, "Attempted to serialize a collection with too many elements.");
		Compact(len as u32).encode_to(dest);
		for item in self {
			item.encode_to(dest);
		}
	}
}

impl<T: Decode> Decode for Vec<T> {
	fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
		<Compact<u32>>::decode(input).and_then(move |Compact(len)| {
			let mut r = Vec::with_capacity(len as usize);
			for _ in 0..len {
				r.push(T::decode(input)?);
			}
			Ok(r)
		})
	}
}

impl<K: Encode + Ord, V: Encode> Encode for BTreeMap<K, V> {
	fn encode_to<W: Output>(&self, dest: &mut W) {
		let len = self.len();
		assert!(len <= u32::max_value() as usize, "Attempted to serialize a collection with too many elements.");
		(len as u32).encode_to(dest);
		for i in self.iter() {
			i.encode_to(dest);
		}
	}
}

impl<K: Decode + Ord, V: Decode> Decode for BTreeMap<K, V> {
	fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
		u32::decode(input).and_then(move |len| {
			let mut r: BTreeMap<K, V> = BTreeMap::new();
			for _ in 0..len {
				let (key, v) = <(K, V)>::decode(input)?;
				r.insert(key, v);
			}
			Ok(r)
		})
	}
}

impl Encode for () {
	fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
		f(&[])
	}
}

impl Decode for () {
	fn decode<I: Input>(_: &mut I) -> Result<(), Error> {
		Ok(())
	}
}

macro_rules! tuple_impl {
	($one:ident,) => {
		impl<$one: Encode> Encode for ($one,) {
			fn size_hint(&self) -> usize {
				self.0.size_hint()
			}

			fn encode_to<T: Output>(&self, dest: &mut T) {
				self.0.encode_to(dest);
			}

			fn encode(&self) -> Vec<u8> {
				self.0.encode()
			}

			fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
				self.0.using_encoded(f)
			}
		}

		impl<$one: Decode> Decode for ($one,) {
			fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
				match $one::decode(input) {
					Err(e) => Err(e),
					Ok($one) => Ok(($one,)),
				}
			}
		}
	};
	($first:ident, $($rest:ident,)+) => {
		impl<$first: Encode, $($rest: Encode),+>
		Encode for
		($first, $($rest),+) {
			fn size_hint(&self) -> usize {
				let (
					ref $first,
					$(ref $rest),+
				) = *self;
				$first.size_hint()
				$( + $rest.size_hint() )+
			}

			fn encode_to<T: Output>(&self, dest: &mut T) {
				let (
					ref $first,
					$(ref $rest),+
				) = *self;

				$first.encode_to(dest);
				$($rest.encode_to(dest);)+
			}
		}

		impl<$first: Decode, $($rest: Decode),+>
		Decode for
		($first, $($rest),+) {
			fn decode<INPUT: Input>(input: &mut INPUT) -> Result<Self, super::Error> {
				Ok((
					match $first::decode(input) {
						Ok(x) => x,
						Err(e) => return Err(e),
					},
					$(match $rest::decode(input) {
						Ok(x) => x,
						Err(e) => return Err(e),
					},)+
				))
			}
		}

		tuple_impl!($($rest,)+);
	}
}

#[allow(non_snake_case)]
mod inner_tuple_impl {
	use super::{Error, Input, Output, Decode, Encode};
	tuple_impl!(A, B, C, D, E, F, G, H, I, J, K,);
}

/// Trait to allow conversion to a know endian representation when sensitive.
/// Types implementing this trait must have a size > 0.
///
/// # Note
///
/// The copy bound and static lifetimes are necessary for safety of `Codec` blanket
/// implementation.
trait EndianSensitive: Copy + 'static {
	fn to_le(self) -> Self { self }
	fn to_be(self) -> Self { self }
	fn from_le(self) -> Self { self }
	fn from_be(self) -> Self { self }
	fn as_be_then<T, F: FnOnce(&Self) -> T>(&self, f: F) -> T { f(&self) }
	fn as_le_then<T, F: FnOnce(&Self) -> T>(&self, f: F) -> T { f(&self) }
}

macro_rules! impl_endians {
	( $( $t:ty ),* ) => { $(
		impl EndianSensitive for $t {
			fn to_le(self) -> Self { <$t>::to_le(self) }
			fn to_be(self) -> Self { <$t>::to_be(self) }
			fn from_le(self) -> Self { <$t>::from_le(self) }
			fn from_be(self) -> Self { <$t>::from_be(self) }
			fn as_be_then<T, F: FnOnce(&Self) -> T>(&self, f: F) -> T { let d = self.to_be(); f(&d) }
			fn as_le_then<T, F: FnOnce(&Self) -> T>(&self, f: F) -> T { let d = self.to_le(); f(&d) }
		}

		impl Encode for $t {
			fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
				self.as_le_then(|le| {
					let size = mem::size_of::<$t>();
					let value_slice = unsafe {
						let ptr = le as *const _ as *const u8;
						if size != 0 {
							slice::from_raw_parts(ptr, size)
						} else {
							&[]
						}
					};

					f(value_slice)
				})
			}
		}

		impl Decode for $t {
			fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
				let size = mem::size_of::<$t>();
				assert!(size > 0, "EndianSensitive can never be implemented for a zero-sized type.");
				let mut val: $t = unsafe { mem::zeroed() };

				unsafe {
					let raw: &mut [u8] = slice::from_raw_parts_mut(
						&mut val as *mut $t as *mut u8,
						size
					);
					input.read(raw)?;
				}
				Ok(val.from_le())
			}
		}
	)* }
}
macro_rules! impl_non_endians {
	( $( $t:ty ),* ) => { $(
		impl EndianSensitive for $t {}

		impl Encode for $t {
			fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
				self.as_le_then(|le| {
					let size = mem::size_of::<$t>();
					let value_slice = unsafe {
						let ptr = le as *const _ as *const u8;
						if size != 0 {
							slice::from_raw_parts(ptr, size)
						} else {
							&[]
						}
					};

					f(value_slice)
				})
			}
		}

		impl Decode for $t {
			fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
				let size = mem::size_of::<$t>();
				assert!(size > 0, "EndianSensitive can never be implemented for a zero-sized type.");
				let mut val: $t = unsafe { mem::zeroed() };

				unsafe {
					let raw: &mut [u8] = slice::from_raw_parts_mut(
						&mut val as *mut $t as *mut u8,
						size
					);
					input.read(raw)?;
				}
				Ok(val.from_le())
			}
		}
	)* }
}

impl_endians!(u16, u32, u64, u128, usize, i16, i32, i64, i128, isize);
impl_non_endians!(i8, [u8; 1], [u8; 2], [u8; 3], [u8; 4], [u8; 5], [u8; 6], [u8; 7], [u8; 8],
	[u8; 10], [u8; 12], [u8; 14], [u8; 16], [u8; 20], [u8; 24], [u8; 28], [u8; 32], [u8; 40],
	[u8; 48], [u8; 56], [u8; 64], [u8; 80], [u8; 96], [u8; 112], [u8; 128], bool);


#[cfg(test)]
mod tests {
	use super::*;
	use std::borrow::Cow;

	#[test]
	fn vec_is_slicable() {
		let v = b"Hello world".to_vec();
		v.using_encoded(|ref slice|
			assert_eq!(slice, &b"\x2cHello world")
		);
	}

	#[test]
	fn btree_map_works() {
		let mut m: BTreeMap<u32, Vec<u8>> = BTreeMap::new();
		m.insert(1, b"qwe".to_vec());
		m.insert(2, b"qweasd".to_vec());
		let encoded = m.encode();

		assert_eq!(m, Decode::decode(&mut &encoded[..]).unwrap());

		let mut m: BTreeMap<Vec<u8>, Vec<u8>> = BTreeMap::new();
		m.insert(b"123".to_vec(), b"qwe".to_vec());
		m.insert(b"1234".to_vec(), b"qweasd".to_vec());
		let encoded = m.encode();

		assert_eq!(m, Decode::decode(&mut &encoded[..]).unwrap());

		let mut m: BTreeMap<Vec<u32>, Vec<u8>> = BTreeMap::new();
		m.insert(vec![1, 2, 3], b"qwe".to_vec());
		m.insert(vec![1, 2], b"qweasd".to_vec());
		let encoded = m.encode();

		assert_eq!(m, Decode::decode(&mut &encoded[..]).unwrap());
	}

	#[test]
	fn encode_borrowed_tuple() {
		let x = vec![1u8, 2, 3, 4];
		let y = 128i64;

		let encoded = (&x, &y).encode();

		assert_eq!((x, y), Decode::decode(&mut &encoded[..]).unwrap());
	}

	#[test]
	fn cow_works() {
		let x = &[1u32, 2, 3, 4, 5, 6][..];
		let y = Cow::Borrowed(&x);
		assert_eq!(x.encode(), y.encode());

		let z: Cow<'_, [u32]> = Cow::decode(&mut &x.encode()[..]).unwrap();
		assert_eq!(*z, *x);
	}

	#[test]
	fn cow_string_works() {
		let x = "Hello world!";
		let y = Cow::Borrowed(&x);
		assert_eq!(x.encode(), y.encode());

		let z: Cow<'_, str> = Cow::decode(&mut &x.encode()[..]).unwrap();
		assert_eq!(*z, *x);
	}

	#[test]
	fn compact_128_encoding_works() {
		let tests = [
			(0u128, 1usize), (63, 1), (64, 2), (16383, 2),
			(16384, 4), (1073741823, 4),
			(1073741824, 5), ((1 << 32) - 1, 5),
			(1 << 32, 6), (1 << 40, 7), (1 << 48, 8), ((1 << 56) - 1, 8), (1 << 56, 9), ((1 << 64) - 1, 9),
			(1 << 64, 10), (1 << 72, 11), (1 << 80, 12), (1 << 88, 13), (1 << 96, 14), (1 << 104, 15),
			(1 << 112, 16), ((1 << 120) - 1, 16), (1 << 120, 17), (u128::max_value(), 17)
		];
		for &(n, l) in &tests {
			let encoded = Compact(n as u128).encode();
			assert_eq!(encoded.len(), l);
			assert_eq!(<Compact<u128>>::decode(&mut &encoded[..]).unwrap().0, n);
		}
	}

	#[test]
	fn compact_64_encoding_works() {
		let tests = [
			(0u64, 1usize), (63, 1), (64, 2), (16383, 2),
			(16384, 4), (1073741823, 4),
			(1073741824, 5), ((1 << 32) - 1, 5),
			(1 << 32, 6), (1 << 40, 7), (1 << 48, 8), ((1 << 56) - 1, 8), (1 << 56, 9), (u64::max_value(), 9)
		];
		for &(n, l) in &tests {
			let encoded = Compact(n as u64).encode();
			assert_eq!(encoded.len(), l);
			assert_eq!(<Compact<u64>>::decode(&mut &encoded[..]).unwrap().0, n);
		}
	}

	#[test]
	fn compact_32_encoding_works() {
		let tests = [(0u32, 1usize), (63, 1), (64, 2), (16383, 2), (16384, 4), (1073741823, 4), (1073741824, 5), (u32::max_value(), 5)];
		for &(n, l) in &tests {
			let encoded = Compact(n as u32).encode();
			assert_eq!(encoded.len(), l);
			assert_eq!(<Compact<u32>>::decode(&mut &encoded[..]).unwrap().0, n);
		}
	}

	#[test]
	fn compact_16_encoding_works() {
		let tests = [(0u16, 1usize), (63, 1), (64, 2), (16383, 2), (16384, 4), (65535, 4)];
		for &(n, l) in &tests {
			let encoded = Compact(n as u16).encode();
			assert_eq!(encoded.len(), l);
			assert_eq!(<Compact<u16>>::decode(&mut &encoded[..]).unwrap().0, n);
		}
		assert!(<Compact<u16>>::decode(&mut &Compact(65536u32).encode()[..]).is_err());
	}

	#[test]
	fn compact_8_encoding_works() {
		let tests = [(0u8, 1usize), (63, 1), (64, 2), (255, 2)];
		for &(n, l) in &tests {
			let encoded = Compact(n as u8).encode();
			assert_eq!(encoded.len(), l);
			assert_eq!(<Compact<u8>>::decode(&mut &encoded[..]).unwrap().0, n);
		}
		assert!(<Compact<u8>>::decode(&mut &Compact(256u32).encode()[..]).is_err());
	}

	fn hexify(bytes: &Vec<u8>) -> String {
		bytes.iter().map(|ref b| format!("{:02x}", b)).collect::<Vec<String>>().join(" ")
	}

	#[test]
	fn string_encoded_as_expected() {
		let value = String::from("Hello, World!");
		let encoded = value.encode();
		assert_eq!(hexify(&encoded), "34 48 65 6c 6c 6f 2c 20 57 6f 72 6c 64 21");
		assert_eq!(<String>::decode(&mut &encoded[..]).unwrap(), value);
	}

	#[test]
	fn vec_of_u8_encoded_as_expected() {
		let value = vec![0u8, 1, 1, 2, 3, 5, 8, 13, 21, 34];
		let encoded = value.encode();
		assert_eq!(hexify(&encoded), "28 00 01 01 02 03 05 08 0d 15 22");
		assert_eq!(<Vec<u8>>::decode(&mut &encoded[..]).unwrap(), value);
	}

	#[test]
	fn vec_of_i16_encoded_as_expected() {
		let value = vec![0i16, 1, -1, 2, -2, 3, -3];
		let encoded = value.encode();
		assert_eq!(hexify(&encoded), "1c 00 00 01 00 ff ff 02 00 fe ff 03 00 fd ff");
		assert_eq!(<Vec<i16>>::decode(&mut &encoded[..]).unwrap(), value);
	}

	#[test]
	fn vec_of_option_int_encoded_as_expected() {
		let value = vec![Some(1i8), Some(-1), None];
		let encoded = value.encode();
		assert_eq!(hexify(&encoded), "0c 01 01 01 ff 00");
		assert_eq!(<Vec<Option<i8>>>::decode(&mut &encoded[..]).unwrap(), value);
	}

	#[test]
	fn vec_of_option_bool_encoded_as_expected() {
		let value = vec![OptionBool(Some(true)), OptionBool(Some(false)), OptionBool(None)];
		let encoded = value.encode();
		assert_eq!(hexify(&encoded), "0c 01 02 00");
		assert_eq!(<Vec<OptionBool>>::decode(&mut &encoded[..]).unwrap(), value);
	}

	#[test]
	fn vec_of_string_encoded_as_expected() {
		let value = vec![
			"Hamlet".to_owned(),
			"Война и мир".to_owned(),
			"三国演义".to_owned(),
			"أَلْف لَيْلَة وَلَيْلَة‎".to_owned()
		];
		let encoded = value.encode();
		assert_eq!(hexify(&encoded), "10 18 48 61 6d 6c 65 74 50 d0 92 d0 be d0 b9 d0 bd d0 b0 20 d0 \
			b8 20 d0 bc d0 b8 d1 80 30 e4 b8 89 e5 9b bd e6 bc 94 e4 b9 89 bc d8 a3 d9 8e d9 84 d9 92 \
			d9 81 20 d9 84 d9 8e d9 8a d9 92 d9 84 d9 8e d8 a9 20 d9 88 d9 8e d9 84 d9 8e d9 8a d9 92 \
			d9 84 d9 8e d8 a9 e2 80 8e");
		assert_eq!(<Vec<String>>::decode(&mut &encoded[..]).unwrap(), value);
	}

	#[test]
	fn compact_integers_encoded_as_expected() {
		let tests = [
			(0u64, "00"),
			(63, "fc"),
			(64, "01 01"),
			(16383, "fd ff"),
			(16384, "02 00 01 00"),
			(1073741823, "fe ff ff ff"),
			(1073741824, "03 00 00 00 40"),
			((1 << 32) - 1, "03 ff ff ff ff"),
			(1 << 32, "07 00 00 00 00 01"),
			(1 << 40, "0b 00 00 00 00 00 01"),
			(1 << 48, "0f 00 00 00 00 00 00 01"),
			((1 << 56) - 1, "0f ff ff ff ff ff ff ff"),
			(1 << 56, "13 00 00 00 00 00 00 00 01"),
			(u64::max_value(), "13 ff ff ff ff ff ff ff ff")
		];
		for &(n, s) in &tests {
			// Verify u64 encoding
			let encoded = Compact(n as u64).encode();
			assert_eq!(hexify(&encoded), s);
			assert_eq!(<Compact<u64>>::decode(&mut &encoded[..]).unwrap().0, n);

			// Verify encodings for lower-size uints are compatible with u64 encoding
			if n <= u32::max_value() as u64 {
				assert_eq!(<Compact<u32>>::decode(&mut &encoded[..]).unwrap().0, n as u32);
				let encoded = Compact(n as u32).encode();
				assert_eq!(hexify(&encoded), s);
				assert_eq!(<Compact<u64>>::decode(&mut &encoded[..]).unwrap().0, n as u64);
			}
			if n <= u16::max_value() as u64 {
				assert_eq!(<Compact<u16>>::decode(&mut &encoded[..]).unwrap().0, n as u16);
				let encoded = Compact(n as u16).encode();
				assert_eq!(hexify(&encoded), s);
				assert_eq!(<Compact<u64>>::decode(&mut &encoded[..]).unwrap().0, n as u64);
			}
			if n <= u8::max_value() as u64 {
				assert_eq!(<Compact<u8>>::decode(&mut &encoded[..]).unwrap().0, n as u8);
				let encoded = Compact(n as u8).encode();
				assert_eq!(hexify(&encoded), s);
				assert_eq!(<Compact<u64>>::decode(&mut &encoded[..]).unwrap().0, n as u64);
			}
		}
	}

	#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
	#[derive(PartialEq, Eq, Clone)]
	struct Wrapper(u8);

	impl CompactAs for Wrapper {
		type As = u8;
		fn encode_as(&self) -> &u8 {
			&self.0
		}
		fn decode_from(x: u8) -> Wrapper {
			Wrapper(x)
		}
	}

	impl From<Compact<Wrapper>> for Wrapper {
		fn from(x: Compact<Wrapper>) -> Wrapper {
			x.0
		}
	}

	#[test]
	fn compact_as_8_encoding_works() {
		let tests = [(0u8, 1usize), (63, 1), (64, 2), (255, 2)];
		for &(n, l) in &tests {
			let compact: Compact<Wrapper> = Wrapper(n).into();
			let encoded = compact.encode();
			assert_eq!(encoded.len(), l);
			let decoded = <Compact<Wrapper>>::decode(&mut & encoded[..]).unwrap();
			let wrapper: Wrapper = decoded.into();
			assert_eq!(wrapper, Wrapper(n));
		}
	}

	struct WithCompact<T: HasCompact> {
		_data: T,
	}

	#[test]
	fn compact_as_has_compact() {
		let _data = WithCompact { _data: Wrapper(1) };
	}

	#[test]
	fn compact_using_encoded_arrayvec_size() {
		Compact(std::u8::MAX).using_encoded(|_| {});
		Compact(std::u16::MAX).using_encoded(|_| {});
		Compact(std::u32::MAX).using_encoded(|_| {});
		Compact(std::u64::MAX).using_encoded(|_| {});
		Compact(std::u128::MAX).using_encoded(|_| {});

		CompactRef(&std::u8::MAX).using_encoded(|_| {});
		CompactRef(&std::u16::MAX).using_encoded(|_| {});
		CompactRef(&std::u32::MAX).using_encoded(|_| {});
		CompactRef(&std::u64::MAX).using_encoded(|_| {});
		CompactRef(&std::u128::MAX).using_encoded(|_| {});
	}

	#[derive(Debug, PartialEq)]
	struct MyWrapper(Compact<u32>);
	impl Deref for MyWrapper {
		type Target = Compact<u32>;
		fn deref(&self) -> &Self::Target { &self.0 }
	}
	impl WrapperTypeEncode for MyWrapper {}

	impl From<Compact<u32>> for MyWrapper {
		fn from(c: Compact<u32>) -> Self { MyWrapper(c) }
	}
	impl WrapperTypeDecode for MyWrapper {
		type Wrapped = Compact<u32>;
	}

	#[test]
	fn should_work_for_wrapper_types() {
		let result = vec![0b1100];

		assert_eq!(MyWrapper(3u32.into()).encode(), result);
		assert_eq!(MyWrapper::decode(&mut &*result).unwrap(), MyWrapper(3_u32.into()));
	}
}
