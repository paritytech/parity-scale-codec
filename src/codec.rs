// Copyright 2017-2018 Parity Technologies
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

#[cfg(feature = "std")]
use std::fmt;
use core::{mem, ops::Deref, marker::PhantomData, iter::FromIterator, convert::TryFrom, time::Duration};

use arrayvec::ArrayVec;

use byte_slice_cast::{AsByteSlice, IntoVecOf};

#[cfg(any(feature = "std", feature = "full"))]
use crate::alloc::{
	string::String,
	sync::Arc,
	rc::Rc,
};
use crate::alloc::{
	vec::Vec,
	boxed::Box,
	borrow::{Cow, ToOwned},
	collections::{
		BTreeMap, BTreeSet, VecDeque, LinkedList, BinaryHeap
	}
};
use crate::compact::Compact;
use crate::encode_like::EncodeLike;

const MAX_PREALLOCATION: usize = 4 * 1024;
const A_BILLION: u32 = 1_000_000_000;

/// Descriptive error type
#[cfg(feature = "std")]
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Error(&'static str);

/// Undescriptive error type when compiled for no std
#[cfg(not(feature = "std"))]
#[derive(PartialEq, Eq, Clone, Debug)]
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
	/// Should return the remaining length of the input data. If no information about the input
	/// length is available, `None` should be returned.
	///
	/// The length is used to constrain the preallocation while decoding. Returning a garbage
	/// length can open the doors for a denial of service attack to your application.
	/// Otherwise, returning `None` can decrease the performance of your application.
	fn remaining_len(&mut self) -> Result<Option<usize>, Error>;

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
	fn remaining_len(&mut self) -> Result<Option<usize>, Error> {
		Ok(Some(self.len()))
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

/// Wrapper that implements Input for any `Read` and `Seek` type.
#[cfg(feature = "std")]
pub struct IoReader<R: std::io::Read + std::io::Seek>(pub R);

#[cfg(feature = "std")]
impl<R: std::io::Read + std::io::Seek> Input for IoReader<R> {
	fn remaining_len(&mut self) -> Result<Option<usize>, Error> {
		use std::convert::TryInto;
		use std::io::SeekFrom;

		let old_pos = self.0.seek(SeekFrom::Current(0))?;
		let len = self.0.seek(SeekFrom::End(0))?;

		// Avoid seeking a third time when we were already at the end of the
		// stream. The branch is usually way cheaper than a seek operation.
		if old_pos != len {
			self.0.seek(SeekFrom::Start(old_pos))?;
		}

		len.saturating_sub(old_pos)
			.try_into()
			.map_err(|_| "Input cannot fit into usize length".into())
			.map(Some)
	}

	fn read(&mut self, into: &mut [u8]) -> Result<(), Error> {
		self.0.read_exact(into).map_err(Into::into)
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


/// !INTERNAL USE ONLY!
///
/// This enum provides type information to optimize encoding/decoding by doing fake specialization.
#[doc(hidden)]
#[non_exhaustive]
pub enum TypeInfo {
	/// Default value of [`Encode::TYPE_INFO`] to not require implementors to set this value in the trait.
	Unknown,
	U8,
	I8,
	U16,
	I16,
	U32,
	I32,
	U64,
	I64,
	U128,
	I128,
}

/// Trait that allows zero-copy write of value-references to slices in LE format.
///
/// Implementations should override `using_encoded` for value types and `encode_to` and `size_hint` for allocating types.
/// Wrapper types should override all methods.
pub trait Encode {
	// !INTERNAL USE ONLY!
	// This const helps SCALE to optimize the encoding/decoding by doing fake specialization.
	#[doc(hidden)]
	const TYPE_INFO: TypeInfo = TypeInfo::Unknown;

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

/// Trait that allows the length of a collection to be read, without having
/// to read and decode the entire elements.
pub trait DecodeLength {
	/// Return the number of elements in `self_encoded`.
	fn len(self_encoded: &[u8]) -> Result<usize, Error>;
}

/// Trait that allows zero-copy read of value-references from slices in LE format.
pub trait Decode: Sized {
	// !INTERNAL USE ONLY!
	// This const helps SCALE to optimize the encoding/decoding by doing fake specialization.
	#[doc(hidden)]
	const TYPE_INFO: TypeInfo = TypeInfo::Unknown;

	/// Attempt to deserialise the value from input.
	fn decode<I: Input>(value: &mut I) -> Result<Self, Error>;
}

/// Trait that allows zero-copy read/write of value-references to/from slices in LE format.
pub trait Codec: Decode + Encode {}
impl<S: Decode + Encode> Codec for S {}

/// Trait that bound `EncodeLike` along with `Encode`. Usefull for generic being used in function
/// with `EncodeLike` parameters.
pub trait FullEncode: Encode + EncodeLike {}
impl<S: Encode + EncodeLike> FullEncode for S {}

/// Trait that bound `EncodeLike` along with `Codec`. Usefull for generic being used in function
/// with `EncodeLike` parameters.
pub trait FullCodec: Decode + FullEncode {}
impl<S: Decode + FullEncode> FullCodec for S {}

/// A marker trait for types that wrap other encodable type.
///
/// Such types should not carry any additional information
/// that would require to be encoded, because the encoding
/// is assumed to be the same as the wrapped type.
pub trait WrapperTypeEncode: Deref {}

impl<T: ?Sized> WrapperTypeEncode for Box<T> {}
impl<T: ?Sized + Encode> EncodeLike for Box<T> {}
impl<T: Encode> EncodeLike<T> for Box<T> {}
impl<T: Encode> EncodeLike<Box<T>> for T {}

impl<T: ?Sized> WrapperTypeEncode for &T {}
impl<T: ?Sized + Encode> EncodeLike for &T {}
impl<T: Encode> EncodeLike<T> for &T {}
impl<T: Encode> EncodeLike<&T> for T {}
impl<T: Encode> EncodeLike<T> for &&T {}
impl<T: Encode> EncodeLike<&&T> for T {}

impl<T: ?Sized> WrapperTypeEncode for &mut T {}
impl<T: ?Sized + Encode> EncodeLike for &mut T {}
impl<T: Encode> EncodeLike<T> for &mut T {}
impl<T: Encode> EncodeLike<&mut T> for T {}

impl<'a, T: ToOwned + ?Sized> WrapperTypeEncode for Cow<'a, T> {}
impl<'a, T: ToOwned + Encode + ?Sized> EncodeLike for Cow<'a, T> {}
impl<'a, T: ToOwned + Encode> EncodeLike<T> for Cow<'a, T> {}
impl<'a, T: ToOwned + Encode> EncodeLike<Cow<'a, T>> for T {}

#[cfg(any(feature = "std", feature = "full"))]
mod feature_full_wrapper_type_encode {
	use super::*;

	impl<T: ?Sized> WrapperTypeEncode for Arc<T> {}
	impl<T: ?Sized + Encode> EncodeLike for Arc<T> {}
	impl<T: Encode> EncodeLike<T> for Arc<T> {}
	impl<T: Encode> EncodeLike<Arc<T>> for T {}

	impl<T: ?Sized> WrapperTypeEncode for Rc<T> {}
	impl<T: ?Sized + Encode> EncodeLike for Rc<T> {}
	impl<T: Encode> EncodeLike<T> for Rc<T> {}
	impl<T: Encode> EncodeLike<Rc<T>> for T {}

	impl WrapperTypeEncode for String {}
	impl EncodeLike for String {}
	impl EncodeLike<&str> for String {}
	impl EncodeLike<String> for &str {}
}

impl<T, X> Encode for X where
	T: Encode + ?Sized,
	X: WrapperTypeEncode<Target = T>,
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

/// A macro that matches on a [`TypeInfo`] and expands a given macro per variant.
///
/// The first parameter to the given macro will be the type of variant (e.g. `u8`, `u32`, etc.) and other parameters
/// given to this macro.
///
/// The last parameter is the code that should be executed for the `Unknown` type info.
macro_rules! with_type_info {
	( $type_info:expr, $macro:ident $( ( $( $params:ident ),* ) )?, { $( $unknown_variant:tt )* }, ) => {
		match $type_info {
			TypeInfo::U8 => { $macro!(u8 $( $( , $params )* )? ) },
			TypeInfo::I8 => { $macro!(i8 $( $( , $params )* )? ) },
			TypeInfo::U16 => { $macro!(u16 $( $( , $params )* )? ) },
			TypeInfo::I16 => { $macro!(i16 $( $( , $params )* )? ) },
			TypeInfo::U32 => { $macro!(u32 $( $( , $params )* )? ) },
			TypeInfo::I32 => { $macro!(i32 $( $( , $params )* )? ) },
			TypeInfo::U64 => { $macro!(u64 $( $( , $params )* )? ) },
			TypeInfo::I64 => { $macro!(i64 $( $( , $params )* )? ) },
			TypeInfo::U128 => { $macro!(u128 $( $( , $params )* )? ) },
			TypeInfo::I128 => { $macro!(i128 $( $( , $params )* )? ) },
			TypeInfo::Unknown => { $( $unknown_variant )* },
		}
	};
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

impl<T, LikeT, E, LikeE> EncodeLike<Result<LikeT, LikeE>> for Result<T, E>
where
	T: EncodeLike<LikeT>,
	LikeT: Encode,
	E: EncodeLike<LikeE>,
	LikeE: Encode,
{}

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

impl EncodeLike for OptionBool {}

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

impl<T: EncodeLike<U>, U: Encode> EncodeLike<Option<U>> for Option<T> {}

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
	( $( $n:expr, )* ) => {
		$(
			impl<T: Encode> Encode for [T; $n] {
				fn size_hint(&self) -> usize {
					mem::size_of::<T>() * $n
				}

				fn encode_to<W: Output>(&self, dest: &mut W) {
					macro_rules! encode_to {
						( u8, $self:ident, $dest:ident ) => {{
							let typed = unsafe { mem::transmute::<&[T], &[u8]>(&$self[..]) };
							$dest.write(&typed)
						}};
						( i8, $self:ident, $dest:ident ) => {{
							// `i8` has the same size as `u8`. We can just convert it here and write to the
							// dest buffer directly.
							let typed = unsafe { mem::transmute::<&[T], &[u8]>(&$self[..]) };
							$dest.write(&typed)
						}};
						( $ty:ty, $self:ident, $dest:ident ) => {{
							let typed = unsafe { mem::transmute::<&[T], &[$ty]>(&$self[..]) };
							$dest.write(<[$ty] as AsByteSlice<$ty>>::as_byte_slice(typed))
						}};
					}

					with_type_info! {
						<T as Encode>::TYPE_INFO,
						encode_to(self, dest),
						{
							for item in self.iter() {
								item.encode_to(dest);
							}
						},
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

			impl<T: EncodeLike<U>, U: Encode> EncodeLike<[U; $n]> for [T; $n] {}
		)*
	}
}

impl_array!(
	1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
	17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
	32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51,
	52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71,
	72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91,
	92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108,
	109, 110, 111, 112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123, 124,
	125, 126, 127, 128, 129, 130, 131, 132, 133, 134, 135, 136, 137, 138, 139, 140,
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

impl<'a, T: ToOwned + ?Sized> Decode for Cow<'a, T>
	where <T as ToOwned>::Owned: Decode,
{
	fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
		Ok(Cow::Owned(Decode::decode(input)?))
	}
}

impl<T> EncodeLike for PhantomData<T> {}

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
		Self::from_utf8(Vec::decode(input)?).map_err(|_| "Invalid utf8 sequence".into())
	}
}

pub(crate) fn compact_encode_len_to<W: Output>(dest: &mut W, len: usize) -> Result<(), Error> {
	if len > u32::max_value() as usize {
		return Err("Attempted to serialize a collection with too many elements.".into());
	}

	Compact(len as u32).encode_to(dest);
	Ok(())
}

impl<T: Encode> Encode for [T] {
	fn size_hint(&self) -> usize {
		mem::size_of::<u32>() + mem::size_of::<T>() * self.len()
	}

	fn encode_to<W: Output>(&self, dest: &mut W) {
		compact_encode_len_to(dest, self.len()).expect("Compact encodes length");

		macro_rules! encode_to {
			( u8, $self:ident, $dest:ident ) => {{
				let typed = unsafe { mem::transmute::<&[T], &[u8]>($self) };
				$dest.write(&typed)
			}};
			( i8, $self:ident, $dest:ident ) => {{
				// `i8` has the same size as `u8`. We can just convert it here and write to the
				// dest buffer directly.
				let typed = unsafe { mem::transmute::<&[T], &[u8]>($self) };
				$dest.write(&typed)
			}};
			( $ty:ty, $self:ident, $dest:ident ) => {{
				let typed = unsafe { mem::transmute::<&[T], &[$ty]>($self) };
				$dest.write(<[$ty] as AsByteSlice<$ty>>::as_byte_slice(typed))
			}};
		}

		with_type_info! {
			<T as Encode>::TYPE_INFO,
			encode_to(self, dest),
			{
				for item in self {
					item.encode_to(dest);
				}
			},
		}
	}
}

/// Read an `u8` vector from the given input.
fn read_vec_u8<I: Input>(input: &mut I, len: usize) -> Result<Vec<u8>, Error> {
	let input_len = input.remaining_len()?;

	// If there is input len and it cannot be pre-allocated then return directly.
	if input_len.map(|l| l < len).unwrap_or(false) {
		return Err("Not enough data to decode vector".into())
	}

	// Note: we checked that if input_len is some then it can preallocated.
	let r = if input_len.is_some() || len < MAX_PREALLOCATION {
		// Here we pre-allocate the whole buffer.
		let mut r = vec![0; len];
		input.read(&mut r)?;

		r
	} else {
		// Here we pre-allocate only the maximum pre-allocation
		let mut r = vec![];

		let mut remains = len;
		while remains != 0 {
			let len_read = MAX_PREALLOCATION.min(remains);
			let len_filled = r.len();
			r.resize(len_filled + len_read, 0);
			input.read(&mut r[len_filled..])?;
			remains -= len_read;
		}

		r
	};

	Ok(r)
}

impl<T> WrapperTypeEncode for Vec<T> {}
impl<T: EncodeLike<U>, U: Encode> EncodeLike<Vec<U>> for Vec<T> {}
impl<T: EncodeLike<U>, U: Encode> EncodeLike<&[U]> for Vec<T> {}
impl<T: EncodeLike<U>, U: Encode> EncodeLike<Vec<U>> for &[T] {}

impl<T: Decode> Decode for Vec<T> {
	fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
		<Compact<u32>>::decode(input).and_then(move |Compact(len)| {
			let len = len as usize;

			macro_rules! decode {
				( u8, $input:ident, $len:ident ) => {{
					let vec = read_vec_u8($input, $len)?;
					Ok(unsafe { mem::transmute::<Vec<u8>, Vec<T>>(vec) })
				}};
				( i8, $input:ident, $len:ident ) => {{
					let vec = read_vec_u8($input, $len)?;
					Ok(unsafe { mem::transmute::<Vec<u8>, Vec<T>>(vec) })
				}};
				( $ty:ty, $input:ident, $len:ident ) => {{
					let vec = read_vec_u8($input, $len * mem::size_of::<$ty>())?;
					let typed = vec.into_vec_of::<$ty>()
						.map_err(|_| "Failed to convert from `Vec<u8>` to typed vec")?;

					Ok(unsafe { mem::transmute::<Vec<$ty>, Vec<T>>(typed) })
				}};
			}

			with_type_info! {
				<T as Decode>::TYPE_INFO,
				decode(input, len),
				{
					let input_capacity = input.remaining_len()?
						.unwrap_or(MAX_PREALLOCATION)
						.checked_div(mem::size_of::<T>())
						.unwrap_or(0);
					let mut r = Vec::with_capacity(input_capacity.min(len));
					for _ in 0..len {
						r.push(T::decode(input)?);
					}
					Ok(r)
				},
			}
		})
	}
}

macro_rules! impl_codec_through_iterator {
	($(
		$type:ident
		{ $( $generics:ident $( : $decode_additional:ident )? ),* }
		{ $( $type_like_generics:ident ),* }
		{ $( $impl_like_generics:tt )* }
	)*) => {$(
		impl<$( $generics: Encode ),*> Encode for $type<$( $generics, )*> {
			fn size_hint(&self) -> usize {
				mem::size_of::<u32>() $( + mem::size_of::<$generics>() * self.len() )*
			}

			fn encode_to<W: Output>(&self, dest: &mut W) {
				compact_encode_len_to(dest, self.len()).expect("Compact encodes length");

				for i in self.iter() {
					i.encode_to(dest);
				}
			}
		}

		impl<$( $generics: Decode $( + $decode_additional )? ),*> Decode
			for $type<$( $generics, )*>
		{
			fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
				<Compact<u32>>::decode(input).and_then(move |Compact(len)| {
					Result::from_iter((0..len).map(|_| Decode::decode(input)))
				})
			}
		}

		impl<$( $impl_like_generics )*> EncodeLike<$type<$( $type_like_generics ),*>>
			for $type<$( $generics ),*> {}
		impl<$( $impl_like_generics )*> EncodeLike<&[( $( $type_like_generics, )* )]>
			for $type<$( $generics ),*> {}
		impl<$( $impl_like_generics )*> EncodeLike<$type<$( $type_like_generics ),*>>
			for &[( $( $generics, )* )] {}
	)*}
}

impl_codec_through_iterator! {
	BTreeMap { K: Ord, V } { LikeK, LikeV}
		{ K: EncodeLike<LikeK>, LikeK: Encode, V: EncodeLike<LikeV>, LikeV: Encode }
	BTreeSet { T: Ord } { LikeT }
		{ T: EncodeLike<LikeT>, LikeT: Encode }
	LinkedList { T } { LikeT }
		{ T: EncodeLike<LikeT>, LikeT: Encode }
	BinaryHeap { T: Ord } { LikeT }
		{ T: EncodeLike<LikeT>, LikeT: Encode }
}

impl<T: Encode> EncodeLike for VecDeque<T> {}
impl<T: EncodeLike<U>, U: Encode> EncodeLike<&[U]> for VecDeque<T> {}
impl<T: EncodeLike<U>, U: Encode> EncodeLike<VecDeque<U>> for &[T] {}
impl<T: EncodeLike<U>, U: Encode> EncodeLike<Vec<U>> for VecDeque<T> {}
impl<T: EncodeLike<U>, U: Encode> EncodeLike<VecDeque<U>> for Vec<T> {}

impl<T: Encode> Encode for VecDeque<T> {
	fn size_hint(&self) -> usize {
		mem::size_of::<u32>() + mem::size_of::<T>() * self.len()
	}

	fn encode_to<W: Output>(&self, dest: &mut W) {
		compact_encode_len_to(dest, self.len()).expect("Compact encodes length");

		macro_rules! encode_to {
			( $ty:ty, $self:ident, $dest:ident ) => {{
				let slices = $self.as_slices();
				let typed = unsafe {
					core::mem::transmute::<(&[T], &[T]), (&[$ty], &[$ty])>(slices)
				};

				$dest.write(<[$ty] as AsByteSlice<$ty>>::as_byte_slice(typed.0));
				$dest.write(<[$ty] as AsByteSlice<$ty>>::as_byte_slice(typed.1));
			}};
		}

		with_type_info! {
			<T as Encode>::TYPE_INFO,
			encode_to(self, dest),
			{
				for item in self {
					item.encode_to(dest);
				}
			},
		}
	}
}

impl<T: Decode> Decode for VecDeque<T> {
	fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
		Ok(<Vec<T>>::decode(input)?.into())
	}
}

impl EncodeLike for () {}

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
					.map_err(|_| "Failed convert decoded size into usize.".into())
			}
		}
	)*}
}

// Collection types that support compact decode length.
impl_len!(Vec<T>, BTreeSet<T>, BTreeMap<K, V>, VecDeque<T>, BinaryHeap<T>, LinkedList<T>);

macro_rules! tuple_impl {
	(
		($one:ident, $extra:ident),
	) => {
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

		impl<$one: EncodeLike<$extra>, $extra: Encode> crate::EncodeLike<($extra,)> for ($one,) {}
	};
	(($first:ident, $fextra:ident), $( ( $rest:ident, $rextra:ident ), )+) => {
		impl<$first: Encode, $($rest: Encode),+> Encode for ($first, $($rest),+) {
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

		impl<$first: Decode, $($rest: Decode),+> Decode for ($first, $($rest),+) {
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

		impl<$first: EncodeLike<$fextra>, $fextra: Encode,
			$($rest: EncodeLike<$rextra>, $rextra: Encode),+> crate::EncodeLike<($fextra, $( $rextra ),+)>
			for ($first, $($rest),+) {}

		tuple_impl!( $( ($rest, $rextra), )+ );
	}
}

#[allow(non_snake_case)]
mod inner_tuple_impl {
	use super::*;

	tuple_impl!(
		(A0, A1), (B0, B1), (C0, C1), (D0, D1), (E0, E1), (F0, F1), (G0, G1), (H0, H1), (I0, I1),
		(J0, J1), (K0, K1), (L0, L1), (M0, M1), (N0, N1), (O0, O1), (P0, P1), (Q0, Q1), (R0, R1),
	);
}

macro_rules! impl_endians {
	( $( $t:ty; $ty_info:ident ),* ) => { $(
		impl EncodeLike for $t {}

		impl Encode for $t {
			const TYPE_INFO: TypeInfo = TypeInfo::$ty_info;

			fn size_hint(&self) -> usize {
				mem::size_of::<$t>()
			}

			fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
				let buf = self.to_le_bytes();
				f(&buf[..])
			}
		}

		impl Decode for $t {
			const TYPE_INFO: TypeInfo = TypeInfo::$ty_info;

			fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
				let mut buf = [0u8; mem::size_of::<$t>()];
				input.read(&mut buf)?;
				Ok(<$t>::from_le_bytes(buf))
			}
		}
	)* }
}
macro_rules! impl_one_byte {
	( $( $t:ty; $ty_info:ident ),* ) => { $(
		impl EncodeLike for $t {}

		impl Encode for $t {
			const TYPE_INFO: TypeInfo = TypeInfo::$ty_info;

			fn size_hint(&self) -> usize {
				mem::size_of::<$t>()
			}

			fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
				f(&[*self as u8][..])
			}
		}

		impl Decode for $t {
			const TYPE_INFO: TypeInfo = TypeInfo::$ty_info;

			fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
				Ok(input.read_byte()? as $t)
			}
		}
	)* }
}

impl_endians!(u16; U16, u32; U32, u64; U64, u128; U128, i16; I16, i32; I32, i64; I64, i128; I128);
impl_one_byte!(u8; U8, i8; I8);

impl EncodeLike for bool {}

impl Encode for bool {
	fn size_hint(&self) -> usize {
		mem::size_of::<bool>()
	}

	fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
		f(&[*self as u8][..])
	}
}

impl Decode for bool {
	fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
		let byte = input.read_byte()?;
		match byte {
			0 => Ok(false),
			1 => Ok(true),
			_ => Err("Invalid boolean representation".into())
		}
	}
}

impl Encode for Duration {
	fn size_hint(&self) -> usize {
		mem::size_of::<u64>() + mem::size_of::<u32>()
	}

	fn encode(&self) -> Vec<u8> {
		let secs = self.as_secs();
		let nanos = self.subsec_nanos();
		(secs, nanos).encode()
	}
}

impl Decode for Duration {
	fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
		let (secs, nanos) = <(u64, u32)>::decode(input)?;
		if nanos >= A_BILLION {
			Err("Number of nanoseconds should not be higher than 10^9.".into())
		} else {
			Ok(Duration::new(secs, nanos))
		}
	}
}

impl EncodeLike for Duration {}

#[cfg(test)]
mod tests {
	use super::*;
	use std::borrow::Cow;

	#[test]
	fn vec_is_sliceable() {
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

	#[test]
	fn io_reader() {
		use std::io::{Seek, SeekFrom};

		let mut io_reader = IoReader(std::io::Cursor::new(&[1u8, 2, 3][..]));

		assert_eq!(io_reader.0.seek(SeekFrom::Current(0)).unwrap(), 0);
		assert_eq!(io_reader.remaining_len().unwrap().unwrap(), 3);

		assert_eq!(io_reader.read_byte().unwrap(), 1);
		assert_eq!(io_reader.0.seek(SeekFrom::Current(0)).unwrap(), 1);
		assert_eq!(io_reader.remaining_len().unwrap().unwrap(), 2);

		assert_eq!(io_reader.read_byte().unwrap(), 2);
		assert_eq!(io_reader.read_byte().unwrap(), 3);
		assert_eq!(io_reader.0.seek(SeekFrom::Current(0)).unwrap(), 3);
		assert_eq!(io_reader.remaining_len().unwrap().unwrap(), 0);
	}

	#[test]
	fn shared_references_implement_encode() {
		std::sync::Arc::new(10u32).encode();
		std::rc::Rc::new(10u32).encode();
	}

	#[test]
	fn not_limit_input_test() {
		use crate::Input;

		struct NoLimit<'a>(&'a [u8]);

		impl<'a> Input for NoLimit<'a> {
			fn remaining_len(&mut self) -> Result<Option<usize>, Error> {
				Ok(None)
			}

			fn read(&mut self, into: &mut [u8]) -> Result<(), Error> {
				self.0.read(into)
			}
		}

		let len = MAX_PREALLOCATION * 2 + 1;
		let mut i = Compact(len as u32).encode();
		i.resize(i.len() + len, 0);
		assert_eq!(<Vec<u8>>::decode(&mut NoLimit(&i[..])).unwrap(), vec![0u8; len]);

		let i = Compact(len as u32).encode();
		assert_eq!(<Vec<u8>>::decode(&mut NoLimit(&i[..])).err().unwrap().what(), "Not enough data to fill buffer");

		let i = Compact(1000u32).encode();
		assert_eq!(<Vec<u8>>::decode(&mut NoLimit(&i[..])).err().unwrap().what(), "Not enough data to fill buffer");
	}

	#[test]
	fn boolean() {
		assert_eq!(true.encode(), vec![1]);
		assert_eq!(false.encode(), vec![0]);
		assert_eq!(bool::decode(&mut &[1][..]).unwrap(), true);
		assert_eq!(bool::decode(&mut &[0][..]).unwrap(), false);
	}

	#[test]
	fn some_encode_like() {
		fn t<B: EncodeLike>() {}
		t::<&[u8]>();
		t::<&str>();
	}

	#[test]
	fn vec_deque_encode_like_vec() {
		let data: VecDeque<u32> = vec![1, 2, 3, 4, 5, 6].into();
		let encoded = data.encode();

		let decoded = Vec::<u32>::decode(&mut &encoded[..]).unwrap();
		assert!(decoded.iter().all(|v| data.contains(&v)));
		assert_eq!(data.len(), decoded.len());

		let encoded = decoded.encode();
		let decoded = VecDeque::<u32>::decode(&mut &encoded[..]).unwrap();
		assert_eq!(data, decoded);
	}

	#[test]
	fn vec_decode_right_capacity() {
		let data: Vec<u32> = vec![1, 2, 3];
		let mut encoded = data.encode();
		encoded.resize(encoded.len() * 2, 0);
		let decoded = Vec::<u32>::decode(&mut &encoded[..]).unwrap();
		assert_eq!(data, decoded);
		assert_eq!(decoded.capacity(), decoded.len());
		// Check with non-integer type
		let data: Vec<String> = vec!["1".into(), "2".into(), "3".into()];
		let mut encoded = data.encode();
		encoded.resize(65536, 0);
		let decoded = Vec::<String>::decode(&mut &encoded[..]).unwrap();
		assert_eq!(data, decoded);
		assert_eq!(decoded.capacity(), decoded.len());
	}

	#[test]
	fn duration() {
		let num_secs = 13;
		let num_nanos = 37;

		let duration = Duration::new(num_secs, num_nanos);
		let expected = (num_secs, num_nanos as u32).encode();

		assert_eq!(duration.encode(), expected);
		assert_eq!(Duration::decode(&mut &expected[..]).unwrap(), duration);
	}

	#[test]
	fn malformed_duration_encoding_fails() {
		// This test should fail, as the number of nanoseconds encoded is exactly 10^9.
		let invalid_nanos = A_BILLION;
		let encoded = (0u64, invalid_nanos).encode();
		assert!(Duration::decode(&mut &encoded[..]).is_err());

		let num_secs = 1u64;
		let num_nanos = 37u32;
		let invalid_nanos = num_secs as u32 * A_BILLION + num_nanos;
		let encoded = (num_secs, invalid_nanos).encode();
		// This test should fail, as the number of nano seconds encoded is bigger than 10^9.
		assert!(Duration::decode(&mut &encoded[..]).is_err());

		// Now constructing a valid duration and encoding it. Those asserts should not fail.
		let duration = Duration::from_nanos(invalid_nanos as u64);
		let expected = (num_secs, num_nanos).encode();

		assert_eq!(duration.encode(), expected);
		assert_eq!(Duration::decode(&mut &expected[..]).unwrap(), duration);
	}

	#[test]
	fn u64_max() {
		let num_secs = u64::max_value();
		let num_nanos = 0;
		let duration = Duration::new(num_secs, num_nanos);
		let expected = (num_secs, num_nanos).encode();

		assert_eq!(duration.encode(), expected);
		assert_eq!(Duration::decode(&mut &expected[..]).unwrap(), duration);
	}

	#[test]
	fn decoding_does_not_overflow() {
		let num_secs = u64::max_value();
		let num_nanos = A_BILLION;

		// `num_nanos`' carry should make `num_secs` overflow if we were to call `Duration::new()`.
		// This test checks that the we do not call `Duration::new()`.
		let encoded = (num_secs, num_nanos).encode();
		assert!(Duration::decode(&mut &encoded[..]).is_err());
	}

	#[test]
	fn string_invalid_utf8() {
		// `167, 10` is not a valid utf8 sequence, so this should be an error.
		let mut bytes: &[u8] = &[20, 114, 167, 10, 20, 114];

		let obj = <String>::decode(&mut bytes);
		assert!(obj.is_err());
	}
}
