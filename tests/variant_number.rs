#[cfg(not(feature="derive"))]
use parity_scale_codec_derive::Encode;
use parity_scale_codec::Encode;

#[test]
fn discriminant_variant_counted_in_default_index() {
	#[derive(Encode)]
	enum T {
		A = 1,
		B,
	}

	assert_eq!(T::A.encode(), vec![1]);
	assert_eq!(T::B.encode(), vec![1]);
}

#[test]
fn skipped_variant_not_counted_in_default_index() {
	#[derive(Encode)]
	enum T {
		#[codec(skip)]
		A,
		B,
	}

	assert_eq!(T::A.encode(), vec![]);
	assert_eq!(T::B.encode(), vec![0]);
}

#[test]
fn index_attr_variant_counted_and_reused_in_default_index() {
	#[derive(Encode)]
	enum T {
		#[codec(index = 1)]
		A,
		B,
	}

	assert_eq!(T::A.encode(), vec![1]);
	assert_eq!(T::B.encode(), vec![1]);
}
