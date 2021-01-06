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

//! `BitVec` specific serialization.

use core::mem;
use bitvec::{vec::BitVec, store::BitStore, order::BitOrder, slice::BitSlice, boxed::BitBox};
use crate::codec::{Encode, Decode, Input, Output, Error, decode_vec_with_len, encode_slice_no_len};
use crate::compact::Compact;
use crate::EncodeLike;

impl<O: BitOrder, T: BitStore + Encode> Encode for BitSlice<O, T> {
	fn encode_to<W: Output>(&self, dest: &mut W) {
		let len = self.len();
		assert!(
			len <= u32::max_value() as usize,
			"Attempted to serialize a collection with too many elements.",
		);
		Compact(len as u32).encode_to(dest);

		let slice = self.as_slice();

		// NOTE: `BitSlice::as_slice` seems to always return the exact number of necessary
		// element `T`, but doc doesn't seem to ensure that it will never contained any
		// useless element `T` at the end.
		// To be safer we cap with the required_items function.
		let slice = &slice[..slice.len().min(required_items::<T>(len))];

		encode_slice_no_len(slice, dest)
	}
}

impl<O: BitOrder, T: BitStore + Encode> Encode for BitVec<O, T> {
	fn encode_to<W: Output>(&self, dest: &mut W) {
		self.as_bitslice().encode_to(dest)
	}
}

impl<O: BitOrder, T: BitStore + Encode> EncodeLike for BitVec<O, T> {}

impl<O: BitOrder, T: BitStore + Decode> Decode for BitVec<O, T> {
	fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
		<Compact<u32>>::decode(input).and_then(move |Compact(bits)| {
			let bits = bits as usize;
			let required_items = required_items::<T>(bits);
			let vec = decode_vec_with_len(input, required_items)?;

			// Otherwise `from_vec` panics.
			if bits > BitSlice::<O, T>::MAX_BITS {
				return Err("Attempt to decode a bitvec with too many bits".into());
			}
			let mut result = Self::from_vec(vec);
			assert!(bits <= result.len());
			unsafe { result.set_len(bits); }
			Ok(result)
		})
	}
}

impl<O: BitOrder, T: BitStore + Encode> Encode for BitBox<O, T> {
	fn encode_to<W: Output>(&self, dest: &mut W) {
		self.as_bitslice().encode_to(dest)
	}
}

impl<O: BitOrder, T: BitStore + Encode> EncodeLike for BitBox<O, T> {}

impl<O: BitOrder, T: BitStore + Decode> Decode for BitBox<O, T> {
	fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
		Ok(Self::from_bitslice(BitVec::<O, T>::decode(input)?.as_bitslice()))
	}
}

/// Calculates the number of item `T` required to store given amount of `bits` as if they were
/// stored in the array of `T`.
fn required_items<T>(bits: usize) -> usize {
	let element_bits = mem::size_of::<T>() * 8;
	(bits + element_bits - 1) / element_bits
}

#[cfg(test)]
mod tests {
	use super::*;
	use bitvec::{bitvec, order::Msb0};
	use crate::codec::MAX_PREALLOCATION;

	macro_rules! test_data {
		($inner_type:ident) => (
			[
				BitVec::<Msb0, $inner_type>::new(),
				bitvec![Msb0, $inner_type; 0],
				bitvec![Msb0, $inner_type; 1],
				bitvec![Msb0, $inner_type; 0, 0],
				bitvec![Msb0, $inner_type; 1, 0],
				bitvec![Msb0, $inner_type; 0, 1],
				bitvec![Msb0, $inner_type; 1, 1],
				bitvec![Msb0, $inner_type; 1, 0, 1],
				bitvec![Msb0, $inner_type; 0, 1, 0, 1, 0, 1, 1],
				bitvec![Msb0, $inner_type; 0, 1, 0, 1, 0, 1, 1, 0],
				bitvec![Msb0, $inner_type; 1, 1, 0, 1, 0, 1, 1, 0, 1],
				bitvec![Msb0, $inner_type; 1, 0, 1, 0, 1, 1, 0, 0, 1, 0, 1, 0, 1, 1, 0],
				bitvec![Msb0, $inner_type; 0, 1, 0, 1, 0, 1, 1, 0, 0, 1, 0, 1, 0, 1, 1, 0],
				bitvec![Msb0, $inner_type; 0, 1, 0, 1, 0, 1, 1, 0, 0, 1, 0, 1, 0, 1, 1, 0, 0],
				bitvec![Msb0, $inner_type; 0; 15],
				bitvec![Msb0, $inner_type; 1; 16],
				bitvec![Msb0, $inner_type; 0; 17],
				bitvec![Msb0, $inner_type; 1; 31],
				bitvec![Msb0, $inner_type; 0; 32],
				bitvec![Msb0, $inner_type; 1; 33],
				bitvec![Msb0, $inner_type; 0; 63],
				bitvec![Msb0, $inner_type; 1; 64],
				bitvec![Msb0, $inner_type; 0; 65],
				bitvec![Msb0, $inner_type; 1; MAX_PREALLOCATION * 8 + 1],
				bitvec![Msb0, $inner_type; 0; MAX_PREALLOCATION * 9],
				bitvec![Msb0, $inner_type; 1; MAX_PREALLOCATION * 32 + 1],
				bitvec![Msb0, $inner_type; 0; MAX_PREALLOCATION * 33],
			]
		)
	}

	#[test]
	fn required_items_test() {
		assert_eq!(0, required_items::<u8>(0));
		assert_eq!(1, required_items::<u8>(1));
		assert_eq!(1, required_items::<u8>(7));
		assert_eq!(1, required_items::<u8>(8));
		assert_eq!(2, required_items::<u8>(9));

		assert_eq!(0, required_items::<u16>(0));
		assert_eq!(1, required_items::<u16>(1));
		assert_eq!(1, required_items::<u16>(15));
		assert_eq!(1, required_items::<u16>(16));
		assert_eq!(2, required_items::<u16>(17));

		assert_eq!(0, required_items::<u32>(0));
		assert_eq!(1, required_items::<u32>(1));
		assert_eq!(1, required_items::<u32>(31));
		assert_eq!(1, required_items::<u32>(32));
		assert_eq!(2, required_items::<u32>(33));

		assert_eq!(0, required_items::<u64>(0));
		assert_eq!(1, required_items::<u64>(1));
		assert_eq!(1, required_items::<u64>(63));
		assert_eq!(1, required_items::<u64>(64));
		assert_eq!(2, required_items::<u64>(65));
	}

	#[test]
	fn bitvec_u8() {
		for v in &test_data!(u8) {
			let encoded = v.encode();
			assert_eq!(*v, BitVec::<Msb0, u8>::decode(&mut &encoded[..]).unwrap());
		}
	}

	#[test]
	fn bitvec_u16() {
		for v in &test_data!(u16) {
			let encoded = v.encode();
			assert_eq!(*v, BitVec::<Msb0, u16>::decode(&mut &encoded[..]).unwrap());
		}
	}

	#[test]
	fn bitvec_u32() {
		for v in &test_data!(u32) {
			let encoded = v.encode();
			assert_eq!(*v, BitVec::<Msb0, u32>::decode(&mut &encoded[..]).unwrap());
		}
	}

	#[test]
	fn bitvec_u64() {
		for v in &test_data!(u64) {
			let encoded = dbg!(v.encode());
			assert_eq!(*v, BitVec::<Msb0, u64>::decode(&mut &encoded[..]).unwrap());
		}
	}

	#[test]
	fn bitslice() {
		let data: &[u8] = &[0x69];
		let slice = BitSlice::<Msb0, u8>::from_slice(data).unwrap();
		let encoded = slice.encode();
		let decoded = BitVec::<Msb0, u8>::decode(&mut &encoded[..]).unwrap();
		assert_eq!(slice, decoded.as_bitslice());
	}

	#[test]
	fn bitbox() {
		let data: &[u8] = &[5, 10];
		let slice = BitSlice::<Msb0, u8>::from_slice(data).unwrap();
		let bb = BitBox::<Msb0, u8>::from_bitslice(slice);
		let encoded = bb.encode();
		let decoded = BitBox::<Msb0, u8>::decode(&mut &encoded[..]).unwrap();
		assert_eq!(bb, decoded);
	}
}
