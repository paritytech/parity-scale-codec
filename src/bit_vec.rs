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

use bitvec::{vec::BitVec, bits::Bits, cursor::Cursor};
use byte_slice_cast::{AsByteSlice, ToByteSlice, FromByteSlice, Error as FromByteSliceError};

use crate::codec::{Encode, Decode, Input, Output, Compact, Error};

impl From<FromByteSliceError> for Error {
	fn from(_: FromByteSliceError) -> Error {
		"failed to cast from byte slice".into()
	}
}

impl<C: Cursor, T: Bits + ToByteSlice> Encode for BitVec<C, T> {
	fn encode_to<W: Output>(&self, dest: &mut W) {
		let len = self.len();
		assert!(len <= u32::max_value() as usize, "Attempted to serialize a collection with too many elements.");
		Compact(len as u32).encode_to(dest);
		dest.write(self.as_slice().as_byte_slice());
	}
}

impl<C: Cursor, T: Bits + FromByteSlice> Decode for BitVec<C, T> {
	fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
		<Compact<u32>>::decode(input).and_then(move |Compact(bits)| {
			let bits = bits as usize;

			let mut vec = vec![0; required_bytes::<T>(bits)];
			input.read(&mut vec)?;

			Ok(if vec.is_empty() {
				Self::new()
			} else {
				let mut result = Self::from_slice(T::from_byte_slice(&vec)?);
				assert!(bits <= result.len());
				unsafe { result.set_len(bits); }
				result
			})
		})
	}
}

// Calculates bytes required to store given amount of `bits` as if they are stored in the array of `T`.
fn required_bytes<T>(bits: usize) -> usize {
	let element_bits = mem::size_of::<T>() * 8;
	(bits + element_bits - 1) / element_bits * mem::size_of::<T>()
}

// TODO: FIXME: bit slice, bitbox, etc.

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
}
