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
		let mut r = Vec::new();
		for _ in 0..L::to_usize() {
			r.push(T::decode(input)?);
		}
		vecarray::VecArray::try_from(r)
			.map_err(|_| "array length does not match definition".into())
	}
}
