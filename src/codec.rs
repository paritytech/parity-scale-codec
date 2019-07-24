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
use crate::alloc::collections::{BTreeMap, BTreeSet, VecDeque, LinkedList, BinaryHeap};
use crate::compact::{Compact, CompactLen};

#[cfg(any(feature = "std", feature = "full"))]
use crate::alloc::{
	string::String,
	borrow::{Cow, ToOwned},
	sync::Arc,
	rc::Rc,
};

use core::{mem, slice, ops::Deref};
use core::marker::PhantomData;
use core::iter::FromIterator;
use arrayvec::ArrayVec;

#[cfg(feature = "std")]
use std::fmt;

use core::convert::TryFrom;

/// Descriptive error type
#[cfg(feature = "std")]
#[derive(PartialEq, Debug)]
pub struct Error(&'static str);

/// Undescriptive error type when compiled for no std
#[cfg(not(feature = "std"))]
#[derive(PartialEq, Debug)]
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
		Error(s)
	}

	#[cfg(not(feature = "std"))]
	fn from(_s: &'static str) -> Error {
		Error
	}
}

/// Trait that allows reading of data into a slice.
pub trait Input {
	/// Return remaining length of input.
	fn remaining_len(&mut self) -> Result<usize, Error>;

	/// Read the exact number of bytes required to fill the given buffer.
	///
	/// Note that this function is similar to `std::io::Read::read_exact` and not
	/// `std::io::Read::read`.
	fn read(&mut self, into: &mut [u8]) -> Result<(), Error>;

	/// Read a single byte from the input.
	fn read_byte(&mut self) -> Result<u8, Error> {
		let mut buf = [0u8];
		self.read(&mut buf[..])?;
		Ok(buf[0])
	}
}

impl<'a> Input for &'a [u8] {
	fn remaining_len(&mut self) -> Result<usize, Error> {
		Ok(self.len())
	}

	fn read(&mut self, into: &mut [u8]) -> Result<(), Error> {
		if into.len() > self.len() {
			return Err("Not enough data to fill buffer".into());
		}
		let len = into.len();
		into.copy_from_slice(&self[..len]);
		*self = &self[len..];
		Ok(())
	}
}

#[cfg(feature = "std")]
impl From<std::io::Error> for Error {
	fn from(err: std::io::Error) -> Self {
		use std::io::ErrorKind::*;
		match err.kind() {
			NotFound => "io error: NotFound".into(),
			PermissionDenied => "io error: PermissionDenied".into(),
			ConnectionRefused => "io error: ConnectionRefused".into(),
			ConnectionReset => "io error: ConnectionReset".into(),
			ConnectionAborted => "io error: ConnectionAborted".into(),
			NotConnected => "io error: NotConnected".into(),
			AddrInUse => "io error: AddrInUse".into(),
			AddrNotAvailable => "io error: AddrNotAvailable".into(),
			BrokenPipe => "io error: BrokenPipe".into(),
			AlreadyExists => "io error: AlreadyExists".into(),
			WouldBlock => "io error: WouldBlock".into(),
			InvalidInput => "io error: InvalidInput".into(),
			InvalidData => "io error: InvalidData".into(),
			TimedOut => "io error: TimedOut".into(),
			WriteZero => "io error: WriteZero".into(),
			Interrupted => "io error: Interrupted".into(),
			Other => "io error: Other".into(),
			UnexpectedEof => "io error: UnexpectedEof".into(),
			_ => "io error: Unkown".into(),
		}
	}
}

#[cfg(feature = "std")]
struct IoReader<R: std::io::Read + std::io::Seek>(R);

#[cfg(feature = "std")]
impl<R: std::io::Read + std::io::Seek> Input for IoReader<R> {
	fn remaining_len(&mut self) -> Result<usize, Error> {
		use std::convert::TryInto;

		let len = self.0.seek(std::io::SeekFrom::End(0))?
			.saturating_sub(self.0.seek(std::io::SeekFrom::Current(0))?);

		Ok(len.try_into().unwrap_or(usize::max_value()))
	}

	fn read(&mut self, into: &mut [u8]) -> Result<(), Error> {
		self.0.read_exact(into)?;
		Ok(())
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

/// This enum must not be exported and must only be instantiable by parity-scale-codec.
/// Because implementation of Encode and Decode for u8 is done in this crate
/// and there is not other usage.
pub enum IsU8 {
	Yes,
	No,
}

/// Trait that allows zero-copy write of value-references to slices in LE format.
///
/// Implementations should override `using_encoded` for value types and `encode_to` and `size_hint` for allocating types.
/// Wrapper types should override all methods.
pub trait Encode {
	#[doc(hidden)]
	// This const is used to optimise implementation of codec for Vec<u8>.
	const IS_U8: IsU8 = IsU8::No;

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

/// Trait that allows to append items to an encoded representation without
/// decoding all previous added items.
pub trait EncodeAppend {
	/// The item that will be appended.
	type Item: Encode;

	/// Append `to_append` items to the given `self_encoded` representation.
	fn append(self_encoded: Vec<u8>, to_append: &[Self::Item]) -> Result<Vec<u8>, Error>;
}

/// Trait that allows the length of a collection to be read, without having
/// to read and decode the entire elements.
pub trait DecodeLength {
	/// Return the number of elements in `self_encoded`.
	fn len(self_encoded: &[u8]) -> Result<usize, Error>;
}

/// Trait that allows zero-copy read of value-references from slices in LE format.
pub trait Decode: Sized {
	#[doc(hidden)]
	const IS_U8: IsU8 = IsU8::No;

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
impl<T: ?Sized> WrapperTypeEncode for Arc<T> {}
#[cfg(any(feature = "std", feature = "full"))]
impl<T: ?Sized> WrapperTypeEncode for Rc<T> {}
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
impl<T> WrapperTypeDecode for Arc<T> {
	type Wrapped = T;
}
#[cfg(any(feature = "std", feature = "full"))]
impl<T> WrapperTypeDecode for Rc<T> {
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

/// Something that can be encoded as a reference.
pub trait EncodeAsRef<'a, T: 'a> {
	/// The reference type that is used for encoding.
	type RefType: Encode + From<&'a T>;
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
	fn size_hint(&self) -> usize {
		1
	}

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
	where <T as ToOwned>::Owned: Decode,
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
	fn size_hint(&self) -> usize {
		if let IsU8::Yes = <T as Encode>::IS_U8 {
			self.len() + mem::size_of::<u32>()
		} else {
			0
		}
	}

	fn encode_to<W: Output>(&self, dest: &mut W) {
		let len = self.len();
		assert!(len <= u32::max_value() as usize, "Attempted to serialize a collection with too many elements.");
		Compact(len as u32).encode_to(dest);
		if let IsU8::Yes= <T as Encode>::IS_U8 {
			let self_transmute = unsafe {
				core::mem::transmute::<&[T], &[u8]>(self)
			};
			dest.write(self_transmute)
		} else {
			for item in self {
				item.encode_to(dest);
			}
		}
	}
}

impl<T: Decode> Decode for Vec<T> {
	fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
		<Compact<u32>>::decode(input).and_then(move |Compact(len)| {
			let len = len as usize;
			let max_preallocation = input.remaining_len()?;

			if let IsU8::Yes = <T as Decode>::IS_U8 {
				let r = if len <= max_preallocation {
					// if len ok for preallocation then preallocate and read the slice
					let mut r = vec![0; len];

					input.read(&mut r[..len])?;
					r
				} else {
					// if len is considered too much for preallocation then use dynamic allocation
					let mut r = Vec::new();
					let mut remains = len;
					let buffer_len = max_preallocation;
					let mut buffer = vec![0; buffer_len];

					while remains != 0 {
						let read_len = buffer_len.min(remains);
						input.read(&mut buffer[..read_len])?;

						remains -= read_len;
						r.extend_from_slice(&buffer[..read_len]);
					}
					r
				};

				let r = unsafe { mem::transmute::<Vec<u8>, Vec<T>>(r) };
				Ok(r)
			} else {
				let capacity = max_preallocation.checked_div(mem::size_of::<T>()).unwrap_or(0);
				let mut r = Vec::with_capacity(capacity);
				for _ in 0..len {
					r.push(T::decode(input)?);
				}
				Ok(r)
			}
		})
	}
}

impl<T: Encode + Decode> EncodeAppend for Vec<T> {
	type Item = T;

	fn append(mut self_encoded: Vec<u8>, to_append: &[Self::Item]) -> Result<Vec<u8>, Error> {
		if self_encoded.is_empty() {
			return Ok(to_append.encode())
		}

		let len = u32::from(Compact::<u32>::decode(&mut &self_encoded[..])?);
		let new_len = len
			.checked_add(to_append.len() as u32)
			.ok_or_else(|| "New vec length greater than `u32::max_value()`.")?;

		let encoded_len = Compact::<u32>::compact_len(&len);
		let encoded_new_len = Compact::<u32>::compact_len(&new_len);

		let replace_len = |dest: &mut Vec<u8>| {
			Compact(new_len).using_encoded(|e| {
				dest[..encoded_new_len].copy_from_slice(e);
			})
		};

		let append_new_elems = |dest: &mut Vec<u8>| to_append.iter().for_each(|a| a.encode_to(dest));

		// If old and new encoded len is equal, we don't need to copy the
		// already encoded data.
		if encoded_len == encoded_new_len {
			replace_len(&mut self_encoded);
			append_new_elems(&mut self_encoded);

			Ok(self_encoded)
		} else {
			let prefix_size = encoded_new_len + self_encoded.len() - encoded_len;
			let size_hint: usize = to_append.iter().map(Encode::size_hint).sum();

			let mut res = Vec::with_capacity(prefix_size + size_hint);
			unsafe { res.set_len(prefix_size); }

			// Insert the new encoded len, copy the already encoded data and
			// add the new element.
			replace_len(&mut res);
			res[encoded_new_len..prefix_size].copy_from_slice(&self_encoded[encoded_len..]);
			append_new_elems(&mut res);

			Ok(res)
		}
	}
}

macro_rules! impl_codec_through_iterator {
	($(
		$type:ty
		{$( $encode_generics:tt )*}
		{$( $decode_generics:tt )*}
	)*) => {$(
		impl<$($encode_generics)*> Encode for $type {
			fn encode_to<W: Output>(&self, dest: &mut W) {
				let len = self.len();
				assert!(len <= u32::max_value() as usize, "Attempted to serialize a collection with too many elements.");
				Compact(len as u32).encode_to(dest);
				for i in self.iter() {
					i.encode_to(dest);
				}
			}
		}

		impl<$($decode_generics)*> Decode for $type {
			fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
				<Compact<u32>>::decode(input).and_then(move |Compact(len)| {
					Result::from_iter((0..len).map(|_| Decode::decode(input)))
				})
			}
		}
	)*}
}

impl_codec_through_iterator! {
	BTreeMap<K, V> { K: Encode , V: Encode } { K: Decode + Ord, V: Decode }
	BTreeSet<T> { T: Encode } { T: Decode + Ord }
	LinkedList<T> { T: Encode } { T: Decode }
	BinaryHeap<T> { T: Encode } { T: Decode + Ord }
}

impl<T: Encode + Ord> Encode for VecDeque<T> {
	fn encode_to<W: Output>(&self, dest: &mut W) {
		let len = self.len();
		assert!(len <= u32::max_value() as usize, "Attempted to serialize a collection with too many elements.");
		Compact(len as u32).encode_to(dest);

		if let IsU8::Yes = <T as Encode>::IS_U8 {
			let slices = self.as_slices();
			let slices_transmute = unsafe {
				core::mem::transmute::<(&[T], &[T]), (&[u8], &[u8])>(slices)
			};
			dest.write(slices_transmute.0);
			dest.write(slices_transmute.1);
		} else {
			for item in self {
				item.encode_to(dest);
			}
		}
	}
}

impl<T: Decode> Decode for VecDeque<T> {
	fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
		Ok(<Vec<T>>::decode(input)?.into())
	}
}

impl Encode for () {
	fn encode_to<W: Output>(&self, _dest: &mut W) {
	}

	fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
		f(&[])
	}

	fn encode(&self) -> Vec<u8> {
		Vec::new()
	}
}

impl Decode for () {
	fn decode<I: Input>(_: &mut I) -> Result<(), Error> {
		Ok(())
	}
}

macro_rules! impl_len {
	( $( $type:ident< $($g:ident),* > ),* ) => { $(
		impl<$($g),*> DecodeLength for $type<$($g),*> {
			fn len(mut self_encoded: &[u8]) -> Result<usize, Error> {
				usize::try_from(u32::from(Compact::<u32>::decode(&mut self_encoded)?))
					.map_err(|_| "Failed convert decded size into usize.".into())
			}
		}
	)*}
}

// Collection types that support compact decode length.
impl_len!(Vec<T>, BTreeSet<T>, BTreeMap<K, V>, VecDeque<T>, BinaryHeap<T>, LinkedList<T>);

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
	use super::{Error, Input, Output, Decode, Encode, Vec};
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
			fn size_hint(&self) -> usize {
				mem::size_of::<$t>()
			}

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
	( $( $t:ty $( { $is_u8:ident } )? ),* ) => { $(
		impl EndianSensitive for $t {}

		impl Encode for $t {
			$( const $is_u8: IsU8 = IsU8::Yes; )?

			fn size_hint(&self) -> usize {
				mem::size_of::<$t>()
			}

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
			$( const $is_u8: IsU8 = IsU8::Yes; )?

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

impl_endians!(u16, u32, u64, u128, i16, i32, i64, i128);
impl_non_endians!(u8 {IS_U8}, i8, bool);


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

	fn hexify(bytes: &[u8]) -> String {
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
	fn vec_encode_append_works() {
		let max_value = 1_000_000;

		let encoded = (0..max_value).fold(Vec::new(), |encoded, v| {
			<Vec<u32> as EncodeAppend>::append(encoded, &[v]).unwrap()
		});

		let decoded = Vec::<u32>::decode(&mut &encoded[..]).unwrap();
		assert_eq!(decoded, (0..max_value).collect::<Vec<_>>());
	}

	#[test]
	fn vec_encode_append_multiple_items_works() {
		let max_value = 1_000_000;

		let encoded = (0..max_value).fold(Vec::new(), |encoded, v| {
			<Vec<u32> as EncodeAppend>::append(encoded, &[v, v, v, v]).unwrap()
		});

		let decoded = Vec::<u32>::decode(&mut &encoded[..]).unwrap();
		let expected = (0..max_value).fold(Vec::new(), |mut vec, i| {
			vec.append(&mut vec![i, i, i, i]);
			vec
		});
		assert_eq!(decoded, expected);
	}

	fn test_encode_length<T: Encode + Decode + DecodeLength>(thing: &T, len: usize) {
		assert_eq!(<T as DecodeLength>::len(&mut &thing.encode()[..]).unwrap(), len);
	}

	#[test]
	fn len_works_for_decode_collection_types() {
		let vector = vec![10; 10];
		let mut btree_map: BTreeMap<u32, u32> = BTreeMap::new();
		btree_map.insert(1, 1);
		btree_map.insert(2, 2);
		let mut btree_set: BTreeSet<u32> = BTreeSet::new();
		btree_set.insert(1);
		btree_set.insert(2);
		let mut vd = VecDeque::new();
		vd.push_front(1);
		vd.push_front(2);
		let mut bh = BinaryHeap::new();
		bh.push(1);
		bh.push(2);
		let mut ll = LinkedList::new();
		ll.push_back(1);
		ll.push_back(2);

		test_encode_length(&vector, 10);
		test_encode_length(&btree_map, 2);
		test_encode_length(&btree_set, 2);
		test_encode_length(&vd, 2);
		test_encode_length(&bh, 2);
		test_encode_length(&ll, 2);
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

	#[test]
	fn codec_vec_deque_u8_and_u16() {
		let mut v_u8 = VecDeque::new();
		let mut v_u16 = VecDeque::new();

		for i in 0..50 {
			v_u8.push_front(i as u8);
			v_u16.push_front(i as u16);
		}
		for i in 50..100 {
			v_u8.push_back(i as u8);
			v_u16.push_back(i as u16);
		}

		assert_eq!(Decode::decode(&mut &v_u8.encode()[..]), Ok(v_u8));
		assert_eq!(Decode::decode(&mut &v_u16.encode()[..]), Ok(v_u16));
	}

	#[test]
	fn codec_iterator() {
		let t1: BTreeSet<u32> = FromIterator::from_iter((0..10).flat_map(|i| 0..i));
		let t2: LinkedList<u32> = FromIterator::from_iter((0..10).flat_map(|i| 0..i));
		let t3: BinaryHeap<u32> = FromIterator::from_iter((0..10).flat_map(|i| 0..i));
		let t4: BTreeMap<u16, u32> = FromIterator::from_iter(
			(0..10)
				.flat_map(|i| 0..i)
				.map(|i| (i as u16, i + 10))
		);
		let t5: BTreeSet<Vec<u8>> = FromIterator::from_iter((0..10).map(|i| Vec::from_iter(0..i)));
		let t6: LinkedList<Vec<u8>> = FromIterator::from_iter((0..10).map(|i| Vec::from_iter(0..i)));
		let t7: BinaryHeap<Vec<u8>> = FromIterator::from_iter((0..10).map(|i| Vec::from_iter(0..i)));
		let t8: BTreeMap<Vec<u8>, u32> = FromIterator::from_iter(
			(0..10)
				.map(|i| Vec::from_iter(0..i))
				.map(|i| (i.clone(), i.len() as u32))
		);

		assert_eq!(Decode::decode(&mut &t1.encode()[..]), Ok(t1));
		assert_eq!(Decode::decode(&mut &t2.encode()[..]), Ok(t2));
		assert_eq!(
			Decode::decode(&mut &t3.encode()[..]).map(BinaryHeap::into_sorted_vec),
			Ok(t3.into_sorted_vec()),
		);
		assert_eq!(Decode::decode(&mut &t4.encode()[..]), Ok(t4));
		assert_eq!(Decode::decode(&mut &t5.encode()[..]), Ok(t5));
		assert_eq!(Decode::decode(&mut &t6.encode()[..]), Ok(t6));
		assert_eq!(
			Decode::decode(&mut &t7.encode()[..]).map(BinaryHeap::into_sorted_vec),
			Ok(t7.into_sorted_vec()),
		);
		assert_eq!(Decode::decode(&mut &t8.encode()[..]), Ok(t8));
	}
}
