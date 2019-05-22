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

// TODO: Make implementation generic.
impl Decode for BitVec {
	fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
		<Compact<u32>>::decode(input).and_then(move |Compact(len_in_bits)| {
			let len_in_bits = len_in_bits as usize;
			let len_in_bytes = (len_in_bits as f64 / 8.).ceil() as usize;
			let mut vec = vec![0; len_in_bytes];
			input.read(&mut vec)?;
			let mut result = Self::from_slice(u8::from_byte_slice(&vec)?);
			assert!(len_in_bits <= result.len());
			unsafe { result.set_len(len_in_bits); }
			Ok(result)
		})
	}
}

// TODO: FIXME: bit slice, bitbox, etc.

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn bitvec_u8() {
		use bitvec::bitvec;

		let vecs = [
			// TODO: Add more edge-cases.
			BitVec::new(),
			bitvec![0],
			bitvec![1],
			bitvec![0, 0],
			bitvec![1, 0],
			bitvec![0, 1],
			bitvec![1, 1],
			bitvec![0, 1, 0],
			bitvec![0, 1, 0, 1, 1, 1, 1, 0, 0, 1, 0, 1, 0, 1],
		];

		for v in &vecs {
			let encoded = v.encode();
			assert_eq!(*v, BitVec::decode(&mut &encoded[..]).unwrap());
		}
	}
}
