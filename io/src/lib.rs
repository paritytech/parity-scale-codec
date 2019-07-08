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

//! # Input/Output Trait Definition for Encoding Libraries
//!
//! This library defines `Input` and `Output` traits that can be used for
//! encoding libraries to define their own `Encode` and `Decode` traits.

#![warn(missing_docs)]

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

/// Trait that allows reading of data into a slice.
pub trait Input {
	/// Error type of this input.
	type Error;

	/// Read the exact number of bytes required to fill the given buffer.
	///
	/// Note that this function is similar to `std::io::Read::read_exact` and not
	/// `std::io::Read::read`.
	fn read(&mut self, into: &mut [u8]) -> Result<(), Self::Error>;

	/// Read a single byte from the input.
	fn read_byte(&mut self) -> Result<u8, Self::Error> {
		let mut buf = [0u8];
		self.read(&mut buf[..])?;
		Ok(buf[0])
	}
}

/// Error for slice-based input. Only used in `no_std` environments.
#[cfg(not(feature = "std"))]
#[derive(PartialEq, Eq, Clone)]
pub enum SliceInputError {
	/// Not enough data to fill the buffer.
	NotEnoughData,
}

#[cfg(not(feature = "std"))]
impl<'a> Input for &'a [u8] {
	type Error = SliceInputError;

	fn read(&mut self, into: &mut [u8]) -> Result<(), SliceInputError> {
		if into.len() > self.len() {
			return Err(SliceInputError::NotEnoughData);
		}
		let len = into.len();
		into.copy_from_slice(&self[..len]);
		*self = &self[len..];
		Ok(())
	}
}

#[cfg(feature = "std")]
impl<R: std::io::Read> Input for R {
	type Error = std::io::Error;

	fn read(&mut self, into: &mut [u8]) -> Result<(), std::io::Error> {
		(self as &mut dyn std::io::Read).read_exact(into)?;
		Ok(())
	}
}

/// Trait that allows writing of data.
pub trait Output: Sized {
	/// Write to the output.
	fn write(&mut self, bytes: &[u8]);

	/// Write a single byte to the output.
	fn push_byte(&mut self, byte: u8) {
		self.write(&[byte]);
	}
}

#[cfg(not(feature = "std"))]
impl Output for alloc::vec::Vec<u8> {
	fn write(&mut self, bytes: &[u8]) {
		self.extend_from_slice(bytes)
	}
}

#[cfg(feature = "std")]
impl<W: std::io::Write> Output for W {
	fn write(&mut self, bytes: &[u8]) {
		(self as &mut dyn std::io::Write).write_all(bytes).expect("Codec outputs are infallible");
	}
}
