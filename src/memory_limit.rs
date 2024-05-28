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

use crate::{Decode, Error, Input};

/// The error message returned when the memory limit is reached.
const DECODE_OOM_ERROR: &str = "Out of memory when decoding";

/// Extension trait to [`Decode`] for decoding with a maximum memory consumption.
pub trait DecodeMemLimit: Decode + Sized {
	fn decode_with_mem_limit<I: Input>(limit: MemLimit, input: &mut I) -> Result<Self, Error>;
}

/// An input that additionally tracks memory usage.
pub struct MemTrackingInput<'a, I> {
	/// The actual input.
	pub inner: &'a mut I,

	/// The remaining memory limit.
	pub limit: MemLimit,
}

/// A limit on allocated memory.
pub struct MemLimit {
	/// The remaining memory limit.
	limit: usize,
	/// Memory alignment to be applied before allocating memory.
	align: Option<MemAlignment>,
}

impl MemLimit {
	/// Try to allocate a contiguous chunk of memory.
	pub fn try_alloc(&mut self, size: usize) -> Result<(), Error> {
		let size = self.align.as_ref().map_or(size, |a| a.align(size));

		if let Some(remaining) = self.limit.checked_sub(size) {
			self.limit = remaining;
			Ok(())
		} else {
			Err(DECODE_OOM_ERROR.into())
		}
	}

	/// Maximal possible limit.
	pub fn max() -> Self {
		Self { limit: usize::MAX, align: None }
	}
}

/// Alignment of some amount of memory.
///
/// Normally the word `alignment` is used in the context of a pointer - not an amount of memory, but this is still
/// the most fitting name.
pub enum MemAlignment {
	/// Round up to the next power of two.
	NextPowerOfTwo,
}

impl MemAlignment {
	fn align(&self, size: usize) -> usize {
		match self {
			MemAlignment::NextPowerOfTwo => {
				size.next_power_of_two().max(size)
			},
		}
	}
}

impl <T: Into<usize>> From<T> for MemLimit {
	fn from(limit: T) -> Self {
		Self { limit: limit.into(), align: None }
	}
}

impl<'a, I: Input> Input for MemTrackingInput<'a, I> {
	fn remaining_len(&mut self) -> Result<Option<usize>, Error> {
		self.inner.remaining_len()
	}

	fn read(&mut self, into: &mut [u8]) -> Result<(), Error> {
		self.inner.read(into)
	}

	fn read_byte(&mut self) -> Result<u8, Error> {
		self.inner.read_byte()
	}

	fn descend_ref(&mut self) -> Result<(), Error> {
		self.inner.descend_ref()
	}

	fn ascend_ref(&mut self) {
		self.inner.ascend_ref()
	}

	fn try_alloc(&mut self, size: usize) -> Result<(), Error> {
		self.limit.try_alloc(size)
	}
}

impl<T: Decode> DecodeMemLimit for T {
	fn decode_with_mem_limit<I: Input>(limit: MemLimit, input: &mut I) -> Result<Self, Error> {
		let mut input = MemTrackingInput { inner: input, limit };
		T::decode(&mut input)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::*;
	use core::mem;

	#[test]
	fn decode_with_mem_limit_oom_detected() {
		let bytes = (Compact(1024 as u32), vec![0u32; 1024]).encode();
		let mut input = &bytes[..];

		// Limit is one too small.
		let limit = 4096 + mem::size_of::<Vec<u32>>() - 1;
		let result = <Vec<u32>>::decode_with_mem_limit((limit as usize).into(), &mut input);
		assert_eq!(result, Err("Out of memory when decoding".into()));

		// Now it works:
		let limit = limit + 1;
		let result = <Vec<u32>>::decode_with_mem_limit((limit as usize).into(), &mut input);
		assert_eq!(result, Ok(vec![0u32; 1024]));
	}

	#[test]
	fn decode_with_mem_limit_tuple_oom_detected() {
		// First entry is 1 KiB, second is 4 KiB.
		let data = (vec![0u8; 1024], vec![0u32; 1024]);
		let bytes = data.encode();

		// Limit is one too small.
		let limit = 1024 + 4096 + 2 * mem::size_of::<Vec<u32>>() - 1;
		let result = <(Vec<u8>, Vec<u32>)>::decode_with_mem_limit((limit as usize).into(), &mut &bytes[..]);
		assert_eq!(result, Err("Out of memory when decoding".into()));

		// Now it works:
		let limit = limit + 1;
		let result = <(Vec<u8>, Vec<u32>)>::decode_with_mem_limit((limit as usize).into(), &mut &bytes[..]);
		assert_eq!(result, Ok(data));
	}
}
