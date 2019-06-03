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

use std::mem;

use bitvec::{vec::BitVec, bits::Bits, cursor::Cursor, slice::BitSlice, boxed::BitBox};

use crate::codec::{Encode, Decode, Input, Output, Compact, Error};

impl<C: Cursor, T: Bits + Encode> Encode for BitSlice<C, T> {
	fn encode_to<W: Output>(&self, dest: &mut W) {
		let len = self.len();
		assert!(len <= u32::max_value() as usize, "Attempted to serialize a collection with too many elements.");
		Compact(len as u32).encode_to(dest);

		for item in self.as_slice() {
			item.encode_to(dest);
		}
	}
}

impl<C: Cursor, T: Bits + Encode> Encode for BitVec<C, T> {
	fn encode_to<W: Output>(&self, dest: &mut W) {
		self.as_bitslice().encode_to(dest)
	}
}

impl<C: Cursor, T: Bits + Decode> Decode for BitVec<C, T> {
	fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
		<Compact<u32>>::decode(input).and_then(move |Compact(bits)| {
			let bits = bits as usize;

			let elements = elements_count::<T>(bits);
			let mut vec = Vec::with_capacity(elements);
			for _ in 0..elements {
				vec.push(T::decode(input)?);
			}

			let mut result = Self::from_slice(&vec);
			assert!(bits <= result.len());
			unsafe { result.set_len(bits); }
			Ok(result)
		})
	}
}

impl<C: Cursor, T: Bits + Encode> Encode for BitBox<C, T> {
	fn encode_to<W: Output>(&self, dest: &mut W) {
		self.as_bitslice().encode_to(dest)
	}
}

impl<C: Cursor, T: Bits + Decode> Decode for BitBox<C, T> {
	fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
		Ok(Self::from_bitslice(BitVec::<C, T>::decode(input)?.as_bitslice()))
	}
}

// Calculates the amount of `T` elements required to store given number of `bits`.
fn elements_count<T>(bits: usize) -> usize {
	let element_bits = mem::size_of::<T>() * 8;
	assert!(element_bits > 0);
	(bits + element_bits - 1) / element_bits
}

#[cfg(test)]
mod tests {
	use super::*;
	use bitvec::{bitvec, cursor::BigEndian};

	macro_rules! test_data {
		($inner_type: ty) => (
			[
				BitVec::<BigEndian, $inner_type>::new(),
				bitvec![BigEndian, $inner_type; 0],
				bitvec![BigEndian, $inner_type; 1],
				bitvec![BigEndian, $inner_type; 0, 0],
				bitvec![BigEndian, $inner_type; 1, 0],
				bitvec![BigEndian, $inner_type; 0, 1],
				bitvec![BigEndian, $inner_type; 1, 1],
				bitvec![BigEndian, $inner_type; 1, 0, 1],
				bitvec![BigEndian, $inner_type; 0, 1, 0, 1, 0, 1, 1],
				bitvec![BigEndian, $inner_type; 0, 1, 0, 1, 0, 1, 1, 0],
				bitvec![BigEndian, $inner_type; 1, 1, 0, 1, 0, 1, 1, 0, 1],
				bitvec![BigEndian, $inner_type; 1, 0, 1, 0, 1, 1, 0, 0, 1, 0, 1, 0, 1, 1, 0],
				bitvec![BigEndian, $inner_type; 0, 1, 0, 1, 0, 1, 1, 0, 0, 1, 0, 1, 0, 1, 1, 0],
				bitvec![BigEndian, $inner_type; 0, 1, 0, 1, 0, 1, 1, 0, 0, 1, 0, 1, 0, 1, 1, 0, 0],
				bitvec![BigEndian, $inner_type; 0; 15],
				bitvec![BigEndian, $inner_type; 1; 16],
				bitvec![BigEndian, $inner_type; 0; 17],
				bitvec![BigEndian, $inner_type; 1; 31],
				bitvec![BigEndian, $inner_type; 0; 32],
				bitvec![BigEndian, $inner_type; 1; 33],
				bitvec![BigEndian, $inner_type; 0; 63],
				bitvec![BigEndian, $inner_type; 1; 64],
				bitvec![BigEndian, $inner_type; 0; 65],
			]
		)
	}

	#[test]
	fn elements_count_test() {
		assert_eq!(0, elements_count::<u8>(0));
		assert_eq!(1, elements_count::<u8>(1));
		assert_eq!(1, elements_count::<u8>(7));
		assert_eq!(1, elements_count::<u8>(8));
		assert_eq!(2, elements_count::<u8>(9));

		assert_eq!(0, elements_count::<u16>(0));
		assert_eq!(1, elements_count::<u16>(1));
		assert_eq!(1, elements_count::<u16>(15));
		assert_eq!(1, elements_count::<u16>(16));
		assert_eq!(2, elements_count::<u16>(17));

		assert_eq!(0, elements_count::<u32>(0));
		assert_eq!(1, elements_count::<u32>(1));
		assert_eq!(1, elements_count::<u32>(31));
		assert_eq!(1, elements_count::<u32>(32));
		assert_eq!(2, elements_count::<u32>(33));

		assert_eq!(0, elements_count::<u64>(0));
		assert_eq!(1, elements_count::<u64>(1));
		assert_eq!(1, elements_count::<u64>(63));
		assert_eq!(1, elements_count::<u64>(64));
		assert_eq!(2, elements_count::<u64>(65));
	}

	#[test]
	fn bitvec_u8() {
		for v in &test_data!(u8) {
			let encoded = v.encode();
			assert_eq!(*v, BitVec::<BigEndian, u8>::decode(&mut &encoded[..]).unwrap());
		}
	}

	#[test]
	fn bitvec_u16() {
		for v in &test_data!(u16) {
			let encoded = v.encode();
			assert_eq!(*v, BitVec::<BigEndian, u16>::decode(&mut &encoded[..]).unwrap());
		}
	}

	#[test]
	fn bitvec_u32() {
		for v in &test_data!(u32) {
			let encoded = v.encode();
			assert_eq!(*v, BitVec::<BigEndian, u32>::decode(&mut &encoded[..]).unwrap());
		}
	}

	#[test]
	fn bitvec_u64() {
		for v in &test_data!(u64) {
			let encoded = v.encode();
			assert_eq!(*v, BitVec::<BigEndian, u64>::decode(&mut &encoded[..]).unwrap());
		}
	}

	#[test]
	fn bitslice() {
		let data: &[u8] = &[0x69];
		let slice: &BitSlice = data.into();
		let encoded = slice.encode();
		let decoded = BitVec::<BigEndian, u8>::decode(&mut &encoded[..]).unwrap();
		assert_eq!(slice, decoded.as_bitslice());
	}

	#[test]
	fn bitbox() {
		let data: &[u8] = &[5, 10];
		let bb: BitBox = data.into();
		let encoded = bb.encode();
		let decoded = BitBox::<BigEndian, u8>::decode(&mut &encoded[..]).unwrap();
		assert_eq!(bb, decoded);
	}
}
