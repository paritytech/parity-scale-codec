#[macro_use]
extern crate parity_scale_codec_derive;

use parity_scale_codec::{Encode, Decode, Error};

/// Mock
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

#[test]
fn enum_struct_test() {
	#[derive(PartialEq, Debug, Default)]
	struct UncodecType;

	#[derive(PartialEq, Debug)]
	struct UncodecUndefaultType;

	#[derive(PartialEq, Debug, Encode, Decode)]
	enum Enum<T=UncodecType, S=UncodecUndefaultType> {
		#[codec(skip)]
		A(S),
		B {
			#[codec(skip)]
			_b1: T,
			b2: u32,
		},
		C(
			#[codec(skip)]
			T,
			u32,
		),
	}

	#[derive(PartialEq, Debug, Encode, Decode)]
	struct StructNamed<T=UncodecType> {
		#[codec(skip)]
		a: T,
		b: u32,
	}

	#[derive(PartialEq, Debug, Encode, Decode)]
	struct StructUnnamed<T=UncodecType>(
		#[codec(skip)]
		T,
		u32,
	);

	let ea: Enum = Enum::A(UncodecUndefaultType);
	let eb: Enum = Enum::B { _b1: UncodecType, b2: 1 };
	let ec: Enum = Enum::C(UncodecType, 1);
	let sn = StructNamed { a: UncodecType, b: 1 };
	let su = StructUnnamed(UncodecType, 1);

	assert_eq!(ea.encode(), Vec::new());

	let mut eb_encoded: &[u8] = &eb.encode();
	let mut ec_encoded: &[u8] = &ec.encode();
	let mut sn_encoded: &[u8] = &sn.encode();
	let mut su_encoded: &[u8] = &su.encode();

	assert_eq!(Enum::decode_m(&mut eb_encoded).unwrap(), eb);
	assert_eq!(Enum::decode_m(&mut ec_encoded).unwrap(), ec);
	assert_eq!(StructNamed::decode_m(&mut sn_encoded).unwrap(), sn);
	assert_eq!(StructUnnamed::decode_m(&mut su_encoded).unwrap(), su);
}
