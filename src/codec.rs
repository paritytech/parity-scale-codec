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

use alloc::vec::Vec;
use alloc::boxed::Box;
use core::{mem, slice};
use arrayvec::ArrayVec;

/// Trait that allows reading of data into a slice.
pub trait Input {
	/// Read into the provided input slice. Returns the number of bytes read.
	fn read(&mut self, into: &mut [u8]) -> usize;

	/// Read a single byte from the input.
	fn read_byte(&mut self) -> Option<u8> {
		let mut buf = [0u8];
		match self.read(&mut buf[..]) {
			0 => None,
			1 => Some(buf[0]),
			_ => unreachable!(),
		}
	}
}

#[cfg(not(feature = "std"))]
impl<'a> Input for &'a [u8] {
	fn read(&mut self, into: &mut [u8]) -> usize {
		let len = ::core::cmp::min(into.len(), self.len());
		into[..len].copy_from_slice(&self[..len]);
		*self = &self[len..];
		len
	}
}

#[cfg(feature = "std")]
impl<R: ::std::io::Read> Input for R {
	fn read(&mut self, into: &mut [u8]) -> usize {
		match (self as &mut ::std::io::Read).read_exact(into) {
			Ok(()) => into.len(),
			Err(_) => 0,
		}
	}
}

/// Prefix another input with a byte.
struct PrefixInput<'a, T: 'a> {
	prefix: Option<u8>,
	input: &'a mut T,
}

impl<'a, T: 'a + Input> Input for PrefixInput<'a, T> {
	fn read(&mut self, buffer: &mut [u8]) -> usize {
		match self.prefix.take() {
			Some(v) if buffer.len() > 0 => {
				buffer[0] = v;
				1 + self.input.read(&mut buffer[1..])
			}
			_ => self.input.read(buffer)
		}
	}
}

/// Trait that allows writing of data.
pub trait Output: Sized {
	/// Write to the output.
	fn write(&mut self, bytes: &[u8]);

	fn push_byte(&mut self, byte: u8) {
		self.write(&[byte]);
	}

	fn push<V: Encode + ?Sized>(&mut self, value: &V) {
		value.encode_to(self);
	}
}

#[cfg(not(feature = "std"))]
impl Output for Vec<u8> {
	fn write(&mut self, bytes: &[u8]) {
		self.extend(bytes);
	}
}

#[cfg(feature = "std")]
impl<W: ::std::io::Write> Output for W {
	fn write(&mut self, bytes: &[u8]) {
		(self as &mut ::std::io::Write).write_all(bytes).expect("Codec outputs are infallible");
	}
}

/// Trait that allows zero-copy write of value-references to slices in LE format.
/// Implementations should override `using_encoded` for value types and `encode_to` for allocating types.
pub trait Encode {
	/// Convert self to a slice and append it to the destination.
	fn encode_to<T: Output>(&self, dest: &mut T) {
		self.using_encoded(|buf| dest.write(buf));
	}

	/// Convert self to an owned vector.
	fn encode(&self) -> Vec<u8> {
		let mut r = Vec::new();
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
	fn decode<I: Input>(value: &mut I) -> Option<Self>;
}

/// Trait that allows zero-copy read/write of value-references to/from slices in LE format.
pub trait Codec: Decode + Encode {}

/// Compact-encoded variant of T. This is more space-efficient but less compute-efficient.
pub struct Compact<T>(pub T);

impl<T> From<T> for Compact<T> {
	fn from(x: T) -> Compact<T> { Compact(x) }
}
impl From<Compact<u8>> for u8 {
	fn from(x: Compact<u8>) -> u8 { x.0 }
}
impl From<Compact<u16>> for u16 {
	fn from(x: Compact<u16>) -> u16 { x.0 }
}
impl From<Compact<u32>> for u32 {
	fn from(x: Compact<u32>) -> u32 { x.0 }
}

// compact encoding:
// 0b00 00 00 00 / 00 00 00 00 / 00 00 00 00 / 00 00 00 00
//   xx xx xx 00															(0 ... 2**6 - 1)		(u8)
//   yL yL yL 01 / yH yH yH yL												(2**6 ... 2**14 - 1)	(u8, u16)  low LH high
//   zL zL zL 10 / zM zM zM zL / zM zM zM zM / zH zH zH zM					(2**14 ... 2**30 - 1)	(u16, u32)  low LMMH high
//   nn nn nn 11 [ / zz zz zz zz ]{4 + n}									(2**30 ... 2**536 - 1)	(u32, u64, u128, U256, U512, U520) straight LE-encoded

// Note: we use *LOW BITS* of the LSB in LE encoding to encode the 2 bit key.

impl Encode for Compact<u8> {
	fn encode_to<W: Output>(&self, dest: &mut W) {
		match self.0 {
			0...0b00111111 => dest.push_byte(self.0 << 2),
			_ => (((self.0 as u16) << 2) | 0b01).encode_to(dest),
		}
	}
}

impl Encode for Compact<u16> {
	fn encode_to<W: Output>(&self, dest: &mut W) {
		match self.0 {
			0...0b00111111 => dest.push_byte((self.0 as u8) << 2),
			0...0b00111111_11111111 => ((self.0 << 2) | 0b01).encode_to(dest),
			_ => (((self.0 as u32) << 2) | 0b10).encode_to(dest),
		}
	}
}

impl Encode for Compact<u32> {
	fn encode_to<W: Output>(&self, dest: &mut W) {
		match self.0 {
			0...0b00111111 => dest.push_byte((self.0 as u8) << 2),
			0...0b00111111_11111111 => (((self.0 as u16) << 2) | 0b01).encode_to(dest),
			0...0b00111111_11111111_11111111_11111111 => ((self.0 << 2) | 0b10).encode_to(dest),
			_ => {
				dest.push_byte(0b11);
				self.0.encode_to(dest);
			}
		}
	}
}

impl Encode for Compact<u64> {
	fn encode_to<W: Output>(&self, dest: &mut W) {
		match self.0 {
			0...0b00111111 => dest.push_byte((self.0 as u8) << 2),
			0...0b00111111_11111111 => (((self.0 as u16) << 2) | 0b01).encode_to(dest),
			0...0b00111111_11111111_11111111_11111111 => (((self.0 as u32) << 2) | 0b10).encode_to(dest),
			_ => {
				dest.push_byte(0b11);
				self.0.encode_to(dest);
			}
		}
	}
}

impl Encode for Compact<u128> {
	fn encode_to<W: Output>(&self, dest: &mut W) {
		match self.0 {
			0...0b00111111 => dest.push_byte((self.0 as u8) << 2),
			0...0b00111111_11111111 => (((self.0 as u16) << 2) | 0b01).encode_to(dest),
			0...0b00111111_11111111_11111111_11111111 => (((self.0 as u32) << 2) | 0b10).encode_to(dest),
			_ => {
				dest.push_byte(0b11);
				self.0.encode_to(dest);
			}
		}
	}
}

impl Decode for Compact<u8> {
	fn decode<I: Input>(input: &mut I) -> Option<Self> {
		let prefix = input.read_byte()?;
		Some(Compact(match prefix % 4 {
			0 => prefix as u8 >> 2,
			1 => {
				let x = u16::decode(&mut PrefixInput{prefix: Some(prefix), input})? >> 2;
				if x < 256 {
					x as u8
				} else {
					return None
				}
			}
			_ => return None,
		}))
	}
}

impl Decode for Compact<u16> {
	fn decode<I: Input>(input: &mut I) -> Option<Self> {
		let prefix = input.read_byte()?;
		Some(Compact(match prefix % 4 {
			0 => prefix as u16 >> 2,
			1 => u16::decode(&mut PrefixInput{prefix: Some(prefix), input})? as u16 >> 2,
			2 => {
				let x = u32::decode(&mut PrefixInput{prefix: Some(prefix), input})? >> 2;
				if x < 65536 {
					x as u16
				} else {
					return None
				}
			}
			_ => return None,
		}))
	}
}

impl Decode for Compact<u32> {
	fn decode<I: Input>(input: &mut I) -> Option<Self> {
		let prefix = input.read_byte()?;
		Some(Compact(match prefix % 4 {
			0 => prefix as u32 >> 2,
			1 => u16::decode(&mut PrefixInput{prefix: Some(prefix), input})? as u32 >> 2,
			2 => u32::decode(&mut PrefixInput{prefix: Some(prefix), input})? as u32 >> 2,
			3|_ => {	// |_. yeah, i know.
				if prefix >> 2 == 0 {
					// just 4 bytes. ok.
					u32::decode(input)?
				} else {
					// Out of range for a 32-bit quantity.
					return None
				}
			}
		}))
	}
}

impl Decode for Compact<u64> {
	fn decode<I: Input>(input: &mut I) -> Option<Self> {
		let prefix = input.read_byte()?;
		Some(Compact(match prefix % 4 {
			0 => prefix as u64 >> 2,
			1 => u16::decode(&mut PrefixInput{prefix: Some(prefix), input})? as u64 >> 2,
			2 => u32::decode(&mut PrefixInput{prefix: Some(prefix), input})? as u64 >> 2,
			3|_ => {
				if prefix >> 2 == 0 {
					u64::decode(input)?
				} else {
					return None
				}
			}
		}))
	}
}

impl Decode for Compact<u128> {
	fn decode<I: Input>(input: &mut I) -> Option<Self> {
		let prefix = input.read_byte()?;
		Some(Compact(match prefix % 4 {
			0 => prefix as u128 >> 2,
			1 => u16::decode(&mut PrefixInput{prefix: Some(prefix), input})? as u128 >> 2,
			2 => u32::decode(&mut PrefixInput{prefix: Some(prefix), input})? as u128 >> 2,
			3|_ => {
				if prefix >> 2 == 0 {
					u128::decode(input)?
				} else {
					return None
				}
			}
		}))
	}
}

impl<S: Decode + Encode> Codec for S {}

impl<T: Encode, E: Encode> Encode for Result<T, E> {
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
	fn decode<I: Input>(input: &mut I) -> Option<Self> {
		match input.read_byte()? {
			0 => Some(Ok(T::decode(input)?)),
			1 => Some(Err(E::decode(input)?)),
			_ => None,
		}
	}
}

/// Shim type because we can't do a specialised implementation for `Option<bool>` directly.
pub struct OptionBool(pub Option<bool>);

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
	fn decode<I: Input>(input: &mut I) -> Option<Self> {
		match input.read_byte()? {
			0 => Some(OptionBool(None)),
			1 => Some(OptionBool(Some(true))),
			2 => Some(OptionBool(Some(false))),
			_ => None,
		}
	}
}

impl<T: Encode> Encode for Option<T> {
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
	fn decode<I: Input>(input: &mut I) -> Option<Self> {
		match input.read_byte()? {
			0 => Some(None),
			1 => Some(Some(T::decode(input)?)),
			_ => None,
		}
	}
}

macro_rules! impl_array {
	( $( $n:expr )* ) => { $(
		impl<T: Encode> Encode for [T; $n] {
			fn encode_to<W: Output>(&self, dest: &mut W) {
				for item in self.iter() {
					item.encode_to(dest);
				}
			}
		}

		impl<T: Decode> Decode for [T; $n] {
			fn decode<I: Input>(input: &mut I) -> Option<Self> {
				let mut r = ArrayVec::new();
				for _ in 0..$n {
					r.push(T::decode(input)?);
				}
				r.into_inner().ok()
			}
		}
	)* }
}

impl_array!(1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31 32
	40 48 56 64 72 96 128 160 192 224 256);

impl<T: Encode> Encode for Box<T> {
	fn encode_to<W: Output>(&self, dest: &mut W) {
		self.as_ref().encode_to(dest)
	}
}

impl<T: Decode> Decode for Box<T> {
	fn decode<I: Input>(input: &mut I) -> Option<Self> {
		Some(Box::new(T::decode(input)?))
	}
}

impl Encode for [u8] {
	fn encode_to<W: Output>(&self, dest: &mut W) {
		let len = self.len();
		assert!(len <= u32::max_value() as usize, "Attempted to serialize a collection with too many elements.");
		Compact(len as u32).encode_to(dest);
		dest.write(self)
	}
}

impl Encode for Vec<u8> {
	fn encode_to<W: Output>(&self, dest: &mut W) {
		self.as_slice().encode_to(dest)
	}
}

impl Decode for Vec<u8> {
	fn decode<I: Input>(input: &mut I) -> Option<Self> {
		<Compact<u32>>::decode(input).and_then(move |Compact(len)| {
			let len = len as usize;
			let mut vec = vec![0; len];
			if input.read(&mut vec[..len]) != len {
				None
			} else {
				Some(vec)
			}
		})
	}
}

impl<'a> Encode for &'a str {
	fn encode_to<W: Output>(&self, dest: &mut W) {
		self.as_bytes().encode_to(dest)
	}
}

#[cfg(feature = "std")]
impl<'a, T: ToOwned + ?Sized + 'a> Encode for ::std::borrow::Cow<'a, T> where
	&'a T: Encode,
	<T as ToOwned>::Owned: Encode
{
	fn encode_to<W: Output>(&self, dest: &mut W) {
		match self {
			::std::borrow::Cow::Owned(ref x) => x.encode_to(dest),
			::std::borrow::Cow::Borrowed(x) => x.encode_to(dest),
		}
	}
}

#[cfg(feature = "std")]
impl<'a, T: ToOwned + ?Sized> Decode for ::std::borrow::Cow<'a, T> where
	<T as ToOwned>::Owned: Decode
{
	fn decode<I: Input>(input: &mut I) -> Option<Self> {
		Some(::std::borrow::Cow::Owned(Decode::decode(input)?))
	}
}

#[cfg(feature = "std")]
impl<T> Encode for ::std::marker::PhantomData<T> {
	fn encode_to<W: Output>(&self, _dest: &mut W) {
	}
}

#[cfg(feature = "std")]
impl<T> Decode for ::std::marker::PhantomData<T> {
	fn decode<I: Input>(_input: &mut I) -> Option<Self> {
		Some(::std::marker::PhantomData)
	}
}

#[cfg(feature = "std")]
impl Encode for String {
	fn encode_to<W: Output>(&self, dest: &mut W) {
		self.as_bytes().encode_to(dest)
	}
}

#[cfg(feature = "std")]
impl Decode for String {
	fn decode<I: Input>(input: &mut I) -> Option<Self> {
		Some(Self::from_utf8_lossy(&Vec::decode(input)?).into())
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

impl<T: Encode> Encode for Vec<T> {
	fn encode_to<W: Output>(&self, dest: &mut W) {
		self.as_slice().encode_to(dest)
	}
}

impl<T: Decode> Decode for Vec<T> {
	fn decode<I: Input>(input: &mut I) -> Option<Self> {
		<Compact<u32>>::decode(input).and_then(move |Compact(len)| {
			let mut r = Vec::with_capacity(len as usize);
			for _ in 0..len {
				r.push(T::decode(input)?);
			}
			Some(r)
		})
	}
}

impl Encode for () {
	fn encode_to<T: Output>(&self, _dest: &mut T) {
	}

	fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
		f(&[])
	}

	fn encode(&self) -> Vec<u8> {
		Vec::new()
	}
}

impl<'a, T: 'a + Encode + ?Sized> Encode for &'a T {
	fn encode_to<D: Output>(&self, dest: &mut D) {
		(&**self).encode_to(dest)
	}

	fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
		(&**self).using_encoded(f)
	}

	fn encode(&self) -> Vec<u8> {
		(&**self).encode()
	}
}

impl Decode for () {
	fn decode<I: Input>(_: &mut I) -> Option<()> {
		Some(())
	}
}

macro_rules! tuple_impl {
	($one:ident,) => {
		impl<$one: Encode> Encode for ($one,) {
			fn encode_to<T: Output>(&self, dest: &mut T) {
				self.0.encode_to(dest);
			}
		}

		impl<$one: Decode> Decode for ($one,) {
			fn decode<I: Input>(input: &mut I) -> Option<Self> {
				match $one::decode(input) {
					None => None,
					Some($one) => Some(($one,)),
				}
			}
		}
	};
	($first:ident, $($rest:ident,)+) => {
		impl<$first: Encode, $($rest: Encode),+>
		Encode for
		($first, $($rest),+) {
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
			fn decode<INPUT: Input>(input: &mut INPUT) -> Option<Self> {
				Some((
					match $first::decode(input) {
						Some(x) => x,
						None => return None,
					},
					$(match $rest::decode(input) {
						Some(x) => x,
						None => return None,
					},)+
				))
			}
		}

		tuple_impl!($($rest,)+);
	}
}

#[allow(non_snake_case)]
mod inner_tuple_impl {
	use super::{Input, Output, Decode, Encode};
	tuple_impl!(A, B, C, D, E, F, G, H, I, J, K,);
}

/// Trait to allow conversion to a know endian representation when sensitive.
/// Types implementing this trait must have a size > 0.
// note: the copy bound and static lifetimes are necessary for safety of `Codec` blanket
// implementation.
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
			fn decode<I: Input>(input: &mut I) -> Option<Self> {
				let size = mem::size_of::<$t>();
				assert!(size > 0, "EndianSensitive can never be implemented for a zero-sized type.");
				let mut val: $t = unsafe { mem::zeroed() };

				unsafe {
					let raw: &mut [u8] = slice::from_raw_parts_mut(
						&mut val as *mut $t as *mut u8,
						size
					);
					if input.read(raw) != size { return None }
				}
				Some(val.from_le())
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
			fn decode<I: Input>(input: &mut I) -> Option<Self> {
				let size = mem::size_of::<$t>();
				assert!(size > 0, "EndianSensitive can never be implemented for a zero-sized type.");
				let mut val: $t = unsafe { mem::zeroed() };

				unsafe {
					let raw: &mut [u8] = slice::from_raw_parts_mut(
						&mut val as *mut $t as *mut u8,
						size
					);
					if input.read(raw) != size { return None }
				}
				Some(val.from_le())
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

		let z: Cow<[u32]> = Cow::decode(&mut &x.encode()[..]).unwrap();
		assert_eq!(*z, *x);
	}

	#[test]
	fn cow_string_works() {
		let x = "Hello world!";
		let y = Cow::Borrowed(&x);
		assert_eq!(x.encode(), y.encode());

		let z: Cow<str> = Cow::decode(&mut &x.encode()[..]).unwrap();
		assert_eq!(*z, *x);
	}

	#[test]
	fn compact_128_encoding_works() {
		let tests = [
			(0u128, 1usize), (63, 1), (64, 2), (16383, 2),
			(16384, 4), (1073741823, 4),
			(1073741824, 17), (u64::max_value() as u128, 17), (u128::max_value(), 17),
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
			(1073741824, 9), (u32::max_value() as u64, 9), (u64::max_value(), 9),
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
		assert!(<Compact<u16>>::decode(&mut &Compact(65536u32).encode()[..]).is_none());
	}

	#[test]
	fn compact_8_encoding_works() {
		let tests = [(0u8, 1usize), (63, 1), (64, 2), (255, 2)];
		for &(n, l) in &tests {
			let encoded = Compact(n as u8).encode();
			assert_eq!(encoded.len(), l);
			assert_eq!(<Compact<u8>>::decode(&mut &encoded[..]).unwrap().0, n);
		}
		assert!(<Compact<u8>>::decode(&mut &Compact(256u32).encode()[..]).is_none());
	}
}
