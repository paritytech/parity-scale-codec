// Copyright 2021 Parity Technologies
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

//! Error type, descriptive or undescriptive depending on features.

use crate::alloc::borrow::Cow;


/// Error type.
///
/// Descriptive on `std` environment, with chaining error on `chain-error` environment,
/// underscriptive otherwise.
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Error {
	#[cfg(feature = "chain-error")]
	cause: Option<Box<Error>>,
	#[cfg(feature = "chain-error")]
	desc: Cow<'static, str>,
	#[cfg(all(not(feature = "chain-error"), feature = "std"))]
	desc: &'static str,
}

impl Error {
	/// Chain error message with description.
	///
	/// When compiled with `chain-error` feature, the description is chained, otherwise the
	/// description is ditched.
	pub fn chain(self, desc: impl Into<Cow<'static, str>>) -> Self {
		#[cfg(feature = "chain-error")]
		{
			Self { desc: desc.into(), cause: Some(Box::new(self)) }
		}

		#[cfg(not(feature = "chain-error"))]
		{
			let _ = desc;
			self
		}
	}

	/// Display error with indentation.
	fn display_with_indent(&self, indent: u32, f: &mut core::fmt::Formatter) -> core::fmt::Result {
		#[cfg(feature = "chain-error")]
		{
			for _ in 0..indent {
				f.write_str("\t")?;
			}
			f.write_str(&self.desc)?;
			if let Some(cause) = &self.cause {
				f.write_str(":")?;
				f.write_str("\n")?;
				cause.display_with_indent(indent + 1, f)
			} else {
				f.write_str("\n")?;
				Ok(())
			}
		}

		#[cfg(all(not(feature = "chain-error"), feature = "std"))]
		{
			for _ in 0..indent {
				f.write_str("\t")?;
			}
			f.write_str(&self.desc)
		}

		#[cfg(all(not(feature = "chain-error"), not(feature = "std")))]
		{
			for _ in 0..indent {
				f.write_str("\t")?;
			}
			f.write_str("Codec error")
		}
	}

	/// Error description
	pub fn what(&self) -> Cow<'static, str> {
		#[cfg(feature = "chain-error")]
		{
			format!("{}", self).into()
		}

		#[cfg(all(not(feature = "chain-error"), feature = "std"))]
		{
			self.desc.into()
		}

		#[cfg(all(not(feature = "chain-error"), not(feature = "std")))]
		{
			"Codec error".into()
		}
	}
}

impl core::fmt::Display for Error {
	fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
		self.display_with_indent(0, f)
	}
}

impl From<&'static str> for Error {
	fn from(desc: &'static str) -> Error {
		#[cfg(feature = "chain-error")]
		{
			Error { desc: desc.into(), cause: None }
		}

		#[cfg(all(not(feature = "chain-error"), feature = "std"))]
		{
			Error { desc }
		}

		#[cfg(all(not(feature = "chain-error"), not(feature = "std")))]
		{
			let _ = desc;
			Error {}
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::Error;

	#[test]
	fn test_full_error() {
		let msg: &str = {
			#[cfg(feature = "chain-error")]
			{
				"final type:\n\twrap cause:\n\t\troot cause\n"
			}

			#[cfg(all(not(feature = "chain-error"), feature = "std"))]
			{
				"root cause"
			}

			#[cfg(all(not(feature = "chain-error"), not(feature = "std")))]
			{
				""
			}
		};

		let error = Error::from("root cause").chain("wrap cause").chain("final type");

		assert_eq!(format!("{}", error), msg);

		assert_eq!(error.what(), msg);
	}
}
