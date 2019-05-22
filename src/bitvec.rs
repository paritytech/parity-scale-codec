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

use bitvec::vec::BitVec;

use std::mem;

use crate::codec::{Encode, Decode, Input, Output, Compact, Error};

impl Encode for BitVec {
	fn encode_to<W: Output>(&self, dest: &mut W) {
		let a1 = vec![1u16, 2, 3];
		//dest.write(a1.as_slice());





		let len = self.len();
		assert!(len <= u32::max_value() as usize, "Attempted to serialize a collection with too many elements.");
		Compact(len as u32).encode_to(dest);
		dest.write(self.as_slice());
	}
}

impl Decode for BitVec {
	fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
		<Compact<u32>>::decode(input).and_then(move |Compact(len)| {
			let element_size = (mem::size_of::<u8>() * 8) as f64;
			let len_in_bytes = (len as f64 / element_size).ceil() as usize;
			let mut array = Vec::<u8>::new();
			for _ in 0..len_in_bytes {
				//array.push(Compact::<u8>::decode(input)?.0);
				array.push(input.read_byte()?);
			}
			let mut result = Self::from_slice(array.as_slice());
			assert!(len <= result.len() as u32);
			unsafe { result.set_len(len as usize); }
			Ok(result)


//			// TODO:
//			let array = match array.into_inner() {
//				Ok(a) => a,
//				Err(_) => Err("failed to get inner array from ArrayVec".into()),
//			};



			///////////////////////////////////////////////////////////////////////////////////////////////
			//let mut r = ArrayVec::new();
			//for _ in 0..$n {
//				r.push(T::decode(input)?);
			//}
//			let i = r.into_inner();
//
//			/match i {
//				Ok(a) => Ok(a),
//				Err(_) => Err("failed to get inner array from ArrayVec".into()),
//			}
			///////////////////////////////////////////////////////////////////////////////////////////////



//			let len = len as usize;
//			let mut v = Self::from_vec(Vec::<u8>::decode(input)?);
//			assert!(len <= v.len());
//			unsafe { v.set_len(len); }
//			Ok(v)
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
