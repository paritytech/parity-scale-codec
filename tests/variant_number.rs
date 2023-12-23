use parity_scale_codec_derive::Encode as DeriveEncode;
use parity_scale_codec::Encode;

#[test]
fn discriminant_variant_counted_in_default_index() {
	#[derive(DeriveEncode)]
	enum T {
		A = 1,
		B,
	}

	assert_eq!(T::A.encode(), vec![1]);
	assert_eq!(T::B.encode(), vec![1]);
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
	assert_eq!(T::B.encode(), vec![1]);
}

#[test]
fn different_const_expr_in_index_attr_variant() {
	const MY_CONST_INDEX: u8 = 1;
	const ANOTHER_CONST_INDEX: u8 = 2;

	#[derive(DeriveEncode)]
	enum T {
		#[codec(index = MY_CONST_INDEX)]
		A,
		B,
		#[codec(index = ANOTHER_CONST_INDEX)]
		C,
		#[codec(index = 3)]
		D,
	}

	assert_eq!(T::A.encode(), vec![1]);
	assert_eq!(T::B.encode(), vec![1]);
	assert_eq!(T::C.encode(), vec![2]);
	assert_eq!(T::D.encode(), vec![3]);
}

#[test]
fn complex_const_expr_in_index_attr_variant() {
    const MY_CONST_INDEX: u8 = 1;

    #[derive(DeriveEncode)]
    enum T {
        #[codec(index = MY_CONST_INDEX + 1_u8)]
        A,
    }

    assert_eq!(T::A.encode(), vec![2]);
}
