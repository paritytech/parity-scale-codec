use parity_scale_codec::{Decode, Error};

/// Assert Decode::decode and Decode::skip works
pub fn assert_decode<T>(mut encoded: &[u8], res: Result<T, Error>) where
	T: core::fmt::Debug + Decode + PartialEq,
{
	assert_eq!(Decode::decode(&mut encoded.clone()), res);
	if res.is_ok() {
		assert_eq!(T::skip(&mut encoded), Ok(()));
		assert!(encoded.is_empty());
	}
}
