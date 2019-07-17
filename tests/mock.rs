use parity_scale_codec::{Decode, Error};

/// Mock that assert min_encoded_len is correct for the decoded value.
pub trait DecodeM: Decode {
	fn decode_m(value: &mut &[u8]) -> Result<Self, Error> {
		let len = value.len();
		let res = Self::decode(value);
		if res.is_ok() {
			assert!(len - value.len() >= Self::min_encoded_len());
		}
		res
	}
}

impl<T: Decode> DecodeM for T {}

