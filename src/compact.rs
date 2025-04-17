// Copyright 2019 Parity Technologies
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

//! [Compact encoding](https://docs.substrate.io/v3/advanced/scale-codec/#compactgeneral-integers)

use arrayvec::ArrayVec;

use crate::{
	alloc::vec::Vec,
	codec::{Decode, Encode, EncodeAsRef, Input, Output},
	encode_like::EncodeLike,
	DecodeWithMemTracking, Error,
};

#[cfg(feature = "fuzz")]
use arbitrary::Arbitrary;

struct ArrayVecWrapper<const N: usize>(ArrayVec<u8, N>);

impl<const N: usize> Output for ArrayVecWrapper<N> {
	fn write(&mut self, bytes: &[u8]) {
		let old_len = self.0.len();
		let new_len = old_len + bytes.len();

		assert!(new_len <= self.0.capacity());
		unsafe {
			self.0.set_len(new_len);
		}

		self.0[old_len..new_len].copy_from_slice(bytes);
	}

	fn push_byte(&mut self, byte: u8) {
		self.0.push(byte);
	}
}

/// Something that can return the compact encoded length for a given value.
pub trait CompactLen<T> {
	/// Returns the compact encoded length for the given value.
	fn compact_len(val: &T) -> usize;
}

/// Compact-encoded variant of T. This is more space-efficient but less compute-efficient.
#[derive(Eq, PartialEq, Clone, Copy, Ord, PartialOrd)]
#[cfg_attr(feature = "fuzz", derive(Arbitrary))]
pub struct Compact<T>(pub T);

impl<T> From<T> for Compact<T> {
	fn from(x: T) -> Compact<T> {
		Compact(x)
	}
}

impl<'a, T: Copy> From<&'a T> for Compact<T> {
	fn from(x: &'a T) -> Compact<T> {
		Compact(*x)
	}
}

/// Allow foreign structs to be wrap in Compact
pub trait CompactAs: From<Compact<Self>> {
	/// A compact-encodable type that should be used as the encoding.
	type As;

	/// Returns the compact-encodable type.
	fn encode_as(&self) -> &Self::As;

	/// Decode `Self` from the compact-decoded type.
	fn decode_from(_: Self::As) -> Result<Self, Error>;
}

impl<T> EncodeLike for Compact<T> where for<'a> CompactRef<'a, T>: Encode {}

impl<T> Encode for Compact<T>
where
	for<'a> CompactRef<'a, T>: Encode,
{
	fn size_hint(&self) -> usize {
		CompactRef(&self.0).size_hint()
	}

	fn encode_to<W: Output + ?Sized>(&self, dest: &mut W) {
		CompactRef(&self.0).encode_to(dest)
	}

	fn encode(&self) -> Vec<u8> {
		CompactRef(&self.0).encode()
	}

	fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
		CompactRef(&self.0).using_encoded(f)
	}
}

impl<T> EncodeLike for CompactRef<'_, T>
where
	T: CompactAs,
	for<'b> CompactRef<'b, T::As>: Encode,
{
}

impl<T> Encode for CompactRef<'_, T>
where
	T: CompactAs,
	for<'b> CompactRef<'b, T::As>: Encode,
{
	fn size_hint(&self) -> usize {
		CompactRef(self.0.encode_as()).size_hint()
	}

	fn encode_to<Out: Output + ?Sized>(&self, dest: &mut Out) {
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
		let as_ = Compact::<T::As>::decode(input)?;
		Ok(Compact(<T as CompactAs>::decode_from(as_.0)?))
	}
}

impl<T> DecodeWithMemTracking for Compact<T>
where
	T: CompactAs,
	Compact<T::As>: DecodeWithMemTracking,
{
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
	fn from(x: &'a T) -> Self {
		CompactRef(x)
	}
}

impl<T> core::fmt::Debug for Compact<T>
where
	T: core::fmt::Debug,
{
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		self.0.fmt(f)
	}
}

#[cfg(feature = "serde")]
impl<T> serde::Serialize for Compact<T>
where
	T: serde::Serialize,
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		T::serialize(&self.0, serializer)
	}
}

#[cfg(feature = "serde")]
impl<'de, T> serde::Deserialize<'de> for Compact<T>
where
	T: serde::Deserialize<'de>,
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		T::deserialize(deserializer).map(Compact)
	}
}

/// Trait that tells you if a given type can be encoded/decoded in a compact way.
pub trait HasCompact: Sized {
	/// The compact type; this can be
	type Type: for<'a> EncodeAsRef<'a, Self> + Decode + From<Self> + Into<Self>;
}

impl<'a, T: 'a> EncodeAsRef<'a, T> for Compact<T>
where
	CompactRef<'a, T>: Encode + From<&'a T>,
{
	type RefType = CompactRef<'a, T>;
}

impl<T: 'static> HasCompact for T
where
	Compact<T>: for<'a> EncodeAsRef<'a, T> + Decode + From<Self> + Into<Self>,
{
	type Type = Compact<T>;
}

impl Encode for CompactRef<'_, ()> {
	fn encode_to<W: Output + ?Sized>(&self, _dest: &mut W) {}

	fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
		f(&[])
	}

	fn encode(&self) -> Vec<u8> {
		Vec::new()
	}
}

struct WrappedPrimitive<T>(T);

impl<T> CompactLen<T> for WrappedPrimitive<T>
where
	T: Copy + Into<u64>,
{
	fn compact_len(val: &T) -> usize {
		let x = (*val).into();
		1 + if x == 0 {
			0
		} else if let Some(l) = (0..8).find(|l| 2_u64.pow(7 * l) <= x && x < 2_u64.pow(7 * (l + 1)))
		{
			l
		} else {
			8
		} as usize
	}
}

impl<T> Encode for WrappedPrimitive<T>
where
	T: Copy + Into<u64>,
{
	fn size_hint(&self) -> usize {
		Self::compact_len(&self.0)
	}

	fn encode_to<W: Output + ?Sized>(&self, dest: &mut W) {
		let x = self.0.into();
		if x == 0 {
			dest.push_byte(0);
		} else if let Some(l) = (0..8).find(|l| 2_u64.pow(7 * l) <= x && x < 2_u64.pow(7 * (l + 1)))
		{
			dest.push_byte((2_u64.pow(8) - 2_u64.pow(8 - l) + (x / 2_u64.pow(8 * l))) as u8);
			dest.write(&(x % 2_u64.pow(8 * l)).to_le_bytes()[..l as usize]);
		} else {
			dest.push_byte((2_u64.pow(8) - 1) as u8);
			dest.write(&x.to_le_bytes());
		}
	}

	fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
		let mut r = ArrayVecWrapper(ArrayVec::<u8, 9>::new());
		self.encode_to(&mut r);
		f(&r.0)
	}
}

impl<T> Decode for WrappedPrimitive<T>
where
	T: Copy + TryFrom<u64>,
{
	fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
		const OUT_OF_RANGE: &str = "Out of range";
		let v = match input.read_byte()? {
			0 => 0,
			0xff => u64::decode(input)?,
			b => {
				let l = (0..8).find(|&i| (b & (0b1000_0000 >> i)) == 0).unwrap();
				let mut buf = [0u8; 8];
				input.read(&mut buf[..l])?;
				let rem = (b & ((1 << (7 - l)) - 1)) as u64;
				u64::from_le_bytes(buf) + (rem << (8 * l))
			},
		};
		let v = T::try_from(v).map_err(|_| Error::from(OUT_OF_RANGE))?;
		Ok(Self(v))
	}
}

impl Encode for CompactRef<'_, u8> {
	fn size_hint(&self) -> usize {
		WrappedPrimitive(*self.0).size_hint()
	}

	fn encode_to<W: Output + ?Sized>(&self, dest: &mut W) {
		WrappedPrimitive(*self.0).encode_to(dest)
	}

	fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
		WrappedPrimitive(*self.0).using_encoded(f)
	}
}

impl CompactLen<u8> for Compact<u8> {
	fn compact_len(val: &u8) -> usize {
		WrappedPrimitive::<u8>::compact_len(val)
	}
}

impl Encode for CompactRef<'_, u16> {
	fn size_hint(&self) -> usize {
		WrappedPrimitive(*self.0).size_hint()
	}

	fn encode_to<W: Output + ?Sized>(&self, dest: &mut W) {
		WrappedPrimitive(*self.0).encode_to(dest)
	}

	fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
		WrappedPrimitive(*self.0).using_encoded(f)
	}
}

impl CompactLen<u16> for Compact<u16> {
	fn compact_len(val: &u16) -> usize {
		WrappedPrimitive::<u16>::compact_len(val)
	}
}

impl Encode for CompactRef<'_, u32> {
	fn size_hint(&self) -> usize {
		WrappedPrimitive(*self.0).size_hint()
	}

	fn encode_to<W: Output + ?Sized>(&self, dest: &mut W) {
		WrappedPrimitive(*self.0).encode_to(dest)
	}

	fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
		WrappedPrimitive(*self.0).using_encoded(f)
	}
}

impl CompactLen<u32> for Compact<u32> {
	fn compact_len(val: &u32) -> usize {
		WrappedPrimitive::<u32>::compact_len(val)
	}
}

impl Encode for CompactRef<'_, u64> {
	fn size_hint(&self) -> usize {
		WrappedPrimitive(*self.0).size_hint()
	}

	fn encode_to<W: Output + ?Sized>(&self, dest: &mut W) {
		WrappedPrimitive(*self.0).encode_to(dest)
	}

	fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
		WrappedPrimitive(*self.0).using_encoded(f)
	}
}

impl CompactLen<u64> for Compact<u64> {
	fn compact_len(val: &u64) -> usize {
		WrappedPrimitive::<u64>::compact_len(val)
	}
}

impl Encode for CompactRef<'_, u128> {
	fn size_hint(&self) -> usize {
		Compact::<u128>::compact_len(self.0)
	}

	fn encode_to<W: Output + ?Sized>(&self, dest: &mut W) {
		let l = (*self.0 & u64::MAX as u128) as u64;
		let h = (*self.0 >> 64) as u64;
		WrappedPrimitive::<u64>::encode_to(&WrappedPrimitive(l), dest);
		WrappedPrimitive::<u64>::encode_to(&WrappedPrimitive(h), dest);
	}
}

impl CompactLen<u128> for Compact<u128> {
	fn compact_len(val: &u128) -> usize {
		let l = (*val & u64::MAX as u128) as u64;
		let h = (*val >> 64) as u64;
		Compact::compact_len(&l) + Compact::compact_len(&h)
	}
}

impl Decode for Compact<()> {
	fn decode<I: Input>(_input: &mut I) -> Result<Self, Error> {
		Ok(Compact(()))
	}
}

impl DecodeWithMemTracking for Compact<()> {}

impl Decode for Compact<u8> {
	fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
		WrappedPrimitive::<u8>::decode(input).map(|w| Compact(w.0))
	}
}

impl DecodeWithMemTracking for Compact<u8> {}

impl Decode for Compact<u16> {
	fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
		WrappedPrimitive::<u16>::decode(input).map(|w| Compact(w.0))
	}
}

impl DecodeWithMemTracking for Compact<u16> {}

impl Decode for Compact<u32> {
	fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
		WrappedPrimitive::<u32>::decode(input).map(|w| Compact(w.0))
	}
}

impl DecodeWithMemTracking for Compact<u32> {}

impl Decode for Compact<u64> {
	fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
		WrappedPrimitive::<u64>::decode(input).map(|w| Compact(w.0))
	}
}

impl DecodeWithMemTracking for Compact<u64> {}

impl Decode for Compact<u128> {
	fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
		let l = WrappedPrimitive::<u64>::decode(input).map(|w| Compact(w.0))?.0;
		let h = WrappedPrimitive::<u64>::decode(input).map(|w| Compact(w.0))?.0;
		Ok(Compact((h as u128) << 64 | l as u128))
	}
}

impl DecodeWithMemTracking for Compact<u128> {}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn compact_128_encoding_works() {
		let tests = [
			(0u128, 2),
			(63, 2),
			(64, 2),
			(16383, 3),
			(16384, 4),
			(1073741823, 6),
			(1073741824, 6),
			((1 << 32) - 1, 6),
			(1 << 32, 6),
			(1 << 40, 7), //10
			(1 << 48, 8),
			((1 << 56) - 1, 9),
			(1 << 56, 10),
			((1 << 64) - 1, 10),
			(1 << 64, 2),
			(1 << 72, 3),
			(1 << 80, 4),
			(1 << 88, 5),
			(1 << 96, 6),
			(1 << 104, 7), //20
			(1 << 112, 8),
			((1 << 120) - 1, 17),
			(1 << 120, 10),
			(u128::MAX, 18),
		];
		for &(n, l) in &tests {
			let encoded = Compact(n).encode();
			println!("{}", hex::encode(&encoded));
			assert_eq!(encoded.len(), l);
			assert_eq!(Compact::compact_len(&n), l);
			assert_eq!(<Compact<u128>>::decode(&mut &encoded[..]).unwrap().0, n);
		}
	}

	#[test]
	fn compact_64_encoding_works() {
		let tests = [
			(0u64, 1usize),
			(63, 1),
			(64, 1),
			(16383, 2),
			(16384, 3),
			(1073741823, 5),
			(1073741824, 5),
			((1 << 32) - 1, 5),
			(1 << 32, 5),
			(1 << 40, 6),
			(1 << 48, 7),
			((1 << 56) - 1, 8),
			(1 << 56, 9),
			(u64::MAX, 9),
		];
		for &(n, l) in &tests {
			let encoded = Compact(n).encode();
			assert_eq!(encoded.len(), l);
			assert_eq!(Compact::compact_len(&n), l);
			assert_eq!(<Compact<u64>>::decode(&mut &encoded[..]).unwrap().0, n);
		}
	}

	#[test]
	fn compact_32_encoding_works() {
		let tests = [
			(0u32, 1usize),
			(63, 1),
			(64, 1),
			(16383, 2),
			(16384, 3),
			(1073741823, 5),
			(1073741824, 5),
			(u32::MAX, 5),
		];
		for &(n, l) in &tests {
			let encoded = Compact(n).encode();
			assert_eq!(encoded.len(), l);
			assert_eq!(Compact::compact_len(&n), l);
			assert_eq!(<Compact<u32>>::decode(&mut &encoded[..]).unwrap().0, n);
		}
	}

	#[test]
	fn compact_16_encoding_works() {
		let tests = [(0u16, 1usize), (63, 1), (64, 1), (16383, 2), (16384, 3), (65535, 3)];
		for &(n, l) in &tests {
			let encoded = Compact(n).encode();
			assert_eq!(encoded.len(), l);
			assert_eq!(Compact::compact_len(&n), l);
			assert_eq!(<Compact<u16>>::decode(&mut &encoded[..]).unwrap().0, n);
		}
		assert!(<Compact<u16>>::decode(&mut &Compact(65536u32).encode()[..]).is_err());
	}

	#[test]
	fn compact_8_encoding_works() {
		let tests = [(0u8, 1usize), (63, 1), (64, 1), (255, 2)];
		for &(n, l) in &tests {
			let encoded = Compact(n).encode();
			assert_eq!(encoded.len(), l);
			assert_eq!(Compact::compact_len(&n), l);
			assert_eq!(<Compact<u8>>::decode(&mut &encoded[..]).unwrap().0, n);
		}
		assert!(<Compact<u8>>::decode(&mut &Compact(256u32).encode()[..]).is_err());
	}

	fn hexify(bytes: &[u8]) -> String {
		bytes
			.iter()
			.map(|ref b| format!("{:02x}", b))
			.collect::<Vec<String>>()
			.join(" ")
	}

	#[test]
	fn compact_integers_encoded_as_expected() {
		let tests = [
			(0u64, "00"),
			(63, "3f"),
			(64, "40"),
			(16383, "bf ff"),
			(16384, "c0 00 40"),
			(1073741823, "f0 ff ff ff 3f"),
			(1073741824, "f0 00 00 00 40"),
			((1 << 32) - 1, "f0 ff ff ff ff"),
			(1 << 32, "f1 00 00 00 00"),
			(1 << 40, "f9 00 00 00 00 00"),
			(1 << 48, "fd 00 00 00 00 00 00"),
			((1 << 56) - 1, "fe ff ff ff ff ff ff ff"),
			(1 << 56, "ff 00 00 00 00 00 00 00 01"),
			(u64::MAX, "ff ff ff ff ff ff ff ff ff"),
		];
		for &(n, s) in &tests {
			// Verify u64 encoding
			let encoded = Compact(n).encode();
			assert_eq!(hexify(&encoded), s);
			assert_eq!(<Compact<u64>>::decode(&mut &encoded[..]).unwrap().0, n);

			// Verify encodings for lower-size uints are compatible with u64 encoding
			if n <= u32::MAX as u64 {
				assert_eq!(<Compact<u32>>::decode(&mut &encoded[..]).unwrap().0, n as u32);
				let encoded = Compact(n as u32).encode();
				assert_eq!(hexify(&encoded), s);
				assert_eq!(<Compact<u64>>::decode(&mut &encoded[..]).unwrap().0, n);
			}
			if n <= u16::MAX as u64 {
				assert_eq!(<Compact<u16>>::decode(&mut &encoded[..]).unwrap().0, n as u16);
				let encoded = Compact(n as u16).encode();
				assert_eq!(hexify(&encoded), s);
				assert_eq!(<Compact<u64>>::decode(&mut &encoded[..]).unwrap().0, n);
			}
			if n <= u8::MAX as u64 {
				assert_eq!(<Compact<u8>>::decode(&mut &encoded[..]).unwrap().0, n as u8);
				let encoded = Compact(n as u8).encode();
				assert_eq!(hexify(&encoded), s);
				assert_eq!(<Compact<u64>>::decode(&mut &encoded[..]).unwrap().0, n);
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
		fn decode_from(x: u8) -> Result<Wrapper, Error> {
			Ok(Wrapper(x))
		}
	}

	impl From<Compact<Wrapper>> for Wrapper {
		fn from(x: Compact<Wrapper>) -> Wrapper {
			x.0
		}
	}

	#[test]
	fn compact_as_8_encoding_works() {
		let tests = [(0u8, 1usize), (63, 1), (64, 1), (255, 2)];
		for &(n, l) in &tests {
			let compact: Compact<Wrapper> = Wrapper(n).into();
			let encoded = compact.encode();
			assert_eq!(encoded.len(), l);
			assert_eq!(Compact::compact_len(&n), l);
			let decoded = <Compact<Wrapper>>::decode(&mut &encoded[..]).unwrap();
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
		Compact(u8::MAX).using_encoded(|_| {});
		Compact(u16::MAX).using_encoded(|_| {});
		Compact(u32::MAX).using_encoded(|_| {});
		Compact(u64::MAX).using_encoded(|_| {});
		Compact(u128::MAX).using_encoded(|_| {});

		CompactRef(&u8::MAX).using_encoded(|_| {});
		CompactRef(&u16::MAX).using_encoded(|_| {});
		CompactRef(&u32::MAX).using_encoded(|_| {});
		CompactRef(&u64::MAX).using_encoded(|_| {});
		CompactRef(&u128::MAX).using_encoded(|_| {});
	}

	#[test]
	#[should_panic]
	fn array_vec_output_oob() {
		let mut v = ArrayVecWrapper(ArrayVec::<u8, 4>::new());
		v.write(&[1, 2, 3, 4, 5]);
	}

	#[test]
	fn array_vec_output() {
		let mut v = ArrayVecWrapper(ArrayVec::<u8, 4>::new());
		v.write(&[1, 2, 3, 4]);
	}

	#[test]
	fn compact_u64_test() {
		for a in [
			u64::MAX,
			u64::MAX - 1,
			u64::MAX << 8,
			(u64::MAX << 8) - 1,
			u64::MAX << 16,
			(u64::MAX << 16) - 1,
		]
		.iter()
		{
			let e = Compact::<u64>::encode(&Compact(*a));
			let d = Compact::<u64>::decode(&mut &e[..]).unwrap().0;
			assert_eq!(*a, d);
		}
	}

	#[test]
	fn compact_u128_test() {
		for a in [u64::MAX as u128, (u64::MAX - 10) as u128, u128::MAX, u128::MAX - 10].iter() {
			let e = Compact::<u128>::encode(&Compact(*a));
			let d = Compact::<u128>::decode(&mut &e[..]).unwrap().0;
			assert_eq!(*a, d);
		}
	}

	macro_rules! quick_check_roundtrip {
		( $( $ty:ty : $test:ident ),* ) => {
			$(
				quickcheck::quickcheck! {
					fn $test(v: $ty) -> bool {
						let encoded = Compact(v).encode();
						let deencoded = <Compact<$ty>>::decode(&mut &encoded[..]).unwrap().0;

						v == deencoded
					}
				}
			)*
		}
	}

	quick_check_roundtrip! {
		u8: u8_roundtrip,
		u16: u16_roundtrip,
		u32 : u32_roundtrip,
		u64 : u64_roundtrip,
		u128 : u128_roundtrip
	}
}
