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

use crate::{Encode, Decode, Error, Output, Input};
use crate::alloc::vec::Vec;
use core::convert::TryFrom;

impl<T: Encode, L: typenum::Unsigned> Encode for vecarray::VecArray<T, L> {
	fn encode_to<W: Output>(&self, dest: &mut W) {
		for item in self.iter() {
			item.encode_to(dest);
		}
	}
}

impl<T: Decode, L: typenum::Unsigned> Decode for vecarray::VecArray<T, L> {
	fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
		let mut r = Vec::with_capacity(L::to_usize());
		for _ in 0..L::to_usize() {
			r.push(T::decode(input)?);
		}
		vecarray::VecArray::try_from(r)
			.map_err(|_| "array length does not match definition".into())
	}
}
