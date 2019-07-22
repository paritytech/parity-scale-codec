use crate::{Encode, Decode, Error, Output, Input};
use crate::alloc::vec::Vec;

impl<T: Encode, L: generic_array::ArrayLength<T>> Encode for generic_array::GenericArray<T, L> {
	fn encode_to<W: Output>(&self, dest: &mut W) {
		for item in self.iter() {
			item.encode_to(dest);
		}
	}
}

impl<T: Decode, L: generic_array::ArrayLength<T>> Decode for generic_array::GenericArray<T, L> {
	fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
		let mut r = Vec::with_capacity(L::to_usize());
		for _ in 0..L::to_usize() {
			r.push(T::decode(input)?);
		}
		let i = generic_array::GenericArray::from_exact_iter(r);

		match i {
			Some(a) => Ok(a),
			None => Err("array length does not match definition".into()),
		}
	}
}
