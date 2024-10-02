use parity_scale_codec::Encode;
use parity_scale_codec_derive::Encode as DeriveEncode;

#[test]
fn discriminant_variant_counted_in_default_index() {
	#[derive(DeriveEncode)]
	enum T {
		A = 1,
		B,
	}

	assert_eq!(T::A.encode(), vec![1]);
	assert_eq!(T::B.encode(), vec![2]);
}

#[test]
fn skipped_variant_not_counted_in_default_index() {
	#[derive(DeriveEncode)]
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
	#[derive(DeriveEncode)]
	enum T {
		#[codec(index = 1)]
		A,
		B,
	}

	assert_eq!(T::A.encode(), vec![1]);
	assert_eq!(T::B.encode(), vec![2]);
}
#[test]
fn index_attr_variant_duplicates_indices() {
	// Tests codec index overriding and that variant indexes are without duplicates
	#[derive(DeriveEncode)]
	enum T {
		#[codec(index = 0)]
		A = 1,
		#[codec(index = 1)]
		B = 0,
	}

	assert_eq!(T::A.encode(), vec![0]);
	assert_eq!(T::B.encode(), vec![1]);
}
#[test]
fn dicriminant_resulting_index() {
	// Tests codec index overriding and that variant indexes are without duplicates
	#[derive(DeriveEncode)]
	enum T {
		Z,
		A = 3,
		B,
		C = 6,
		D,
	}
	assert_eq!(T::Z.encode(), vec![0]);
	assert_eq!(T::A.encode(), vec![3]);
	assert_eq!(T::B.encode(), vec![4]);
	assert_eq!(T::C.encode(), vec![6]);
	assert_eq!(T::D.encode(), vec![7]);
}
