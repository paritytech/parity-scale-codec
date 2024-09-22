// Copyright 2017-2024 Parity Technologies
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

/// The value of a counter that has a maximum.
#[derive(Eq, PartialEq, Clone, Debug)]
pub enum Count {
	/// The counter has an exact value.
	Exact(u64),
	/// The counter has reached its maximum countable value.
	MaxCountReached,
}

/// A wrapper for `Input` which tracks the number fo bytes that are successfully read.
///
/// If inner `Input` fails to read, the counter is not incremented.
///
/// It can count until `u64::MAX - 1` accurately.
pub struct CountedInput<'a, I: crate::Input> {
	input: &'a mut I,
	counter: u64,
}

impl<'a, I: crate::Input> CountedInput<'a, I> {
	/// Create a new `CountedInput` with the given input.
	pub fn new(input: &'a mut I) -> Self {
		Self { input, counter: 0 }
	}

	/// Get the number of bytes successfully read.
	/// Count until `u64::MAX - 1` accurately.
	pub fn count(&self) -> Count {
		if self.counter == u64::MAX {
			Count::MaxCountReached
		} else {
			Count::Exact(self.counter)
		}
	}
}

impl<I: crate::Input> crate::Input for CountedInput<'_, I> {
	fn remaining_len(&mut self) -> Result<Option<usize>, crate::Error> {
		self.input.remaining_len()
	}

	fn read(&mut self, into: &mut [u8]) -> Result<(), crate::Error> {
		self.input.read(into)
			.map(|r| {
				self.counter = self.counter.saturating_add(
					into.len().try_into().unwrap_or(u64::MAX)
				);
				r
			})
	}

	fn read_byte(&mut self) -> Result<u8, crate::Error> {
		self.input.read_byte()
			.map(|r| {
				self.counter = self.counter.saturating_add(1);
				r
			})
	}

	fn ascend_ref(&mut self) {
		self.input.ascend_ref()
	}

	fn descend_ref(&mut self) -> Result<(), crate::Error> {
		self.input.descend_ref()
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::Input;

	#[test]
	fn test_counted_input_input_impl() {
		let mut input = &[1u8, 2, 3, 4, 5][..];
		let mut counted_input = CountedInput::new(&mut input);

		assert_eq!(counted_input.remaining_len().unwrap(), Some(5));
		assert_eq!(counted_input.count(), Count::Exact(0));

		counted_input.read_byte().unwrap();

		assert_eq!(counted_input.remaining_len().unwrap(), Some(4));
		assert_eq!(counted_input.count(), Count::Exact(1));

		counted_input.read(&mut [0u8; 2][..]).unwrap();

		assert_eq!(counted_input.remaining_len().unwrap(), Some(2));
		assert_eq!(counted_input.count(), Count::Exact(3));

		counted_input.ascend_ref();
		counted_input.descend_ref().unwrap();

		counted_input.read(&mut [0u8; 2][..]).unwrap();

		assert_eq!(counted_input.remaining_len().unwrap(), Some(0));
		assert_eq!(counted_input.count(), Count::Exact(5));

		assert_eq!(counted_input.read_byte(), Err("Not enough data to fill buffer".into()));

		assert_eq!(counted_input.remaining_len().unwrap(), Some(0));
		assert_eq!(counted_input.count(), Count::Exact(5));

		assert_eq!(counted_input.read(&mut [0u8; 2][..]), Err("Not enough data to fill buffer".into()));

		assert_eq!(counted_input.remaining_len().unwrap(), Some(0));
		assert_eq!(counted_input.count(), Count::Exact(5));
	}

	#[test]
	fn test_counted_input_max_count_read_byte() {
		let max_exact_count = u64::MAX - 1;

		let mut input = &[0u8; 1000][..];
		let mut counted_input = CountedInput::new(&mut input);

		counted_input.counter = max_exact_count - 2;

		assert_eq!(counted_input.count(), Count::Exact(max_exact_count - 2));

		counted_input.read_byte().unwrap();

		assert_eq!(counted_input.count(), Count::Exact(max_exact_count - 1));

		counted_input.read_byte().unwrap();

		assert_eq!(counted_input.count(), Count::Exact(max_exact_count));

		counted_input.read_byte().unwrap();

		assert_eq!(counted_input.count(), Count::MaxCountReached);

		counted_input.read_byte().unwrap();

		assert_eq!(counted_input.count(), Count::MaxCountReached);
	}

	#[test]
	fn test_counted_input_max_count_read() {
		let max_exact_count = u64::MAX - 1;

		let mut input = &[0u8; 1000][..];
		let mut counted_input = CountedInput::new(&mut input);

		counted_input.counter = max_exact_count - 1;

		assert_eq!(counted_input.count(), Count::Exact(max_exact_count - 1));

		counted_input.read_byte().unwrap();

		assert_eq!(counted_input.count(), Count::Exact(max_exact_count));

		counted_input.read(&mut [0u8; 2][..]).unwrap();

		assert_eq!(counted_input.count(), Count::MaxCountReached);

		counted_input.read(&mut [0u8; 2][..]).unwrap();

		assert_eq!(counted_input.count(), Count::MaxCountReached);
	}
}
