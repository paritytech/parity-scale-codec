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

#[macro_use]
extern crate serde_derive;

extern crate parity_codec;

#[macro_use]
extern crate parity_codec_derive;

use parity_codec::{Encode, Decode, HasCompact, Compact, EncodeAsRef, CompactAs};

#[derive(Debug, PartialEq, Encode, Decode)]
struct Unit;

#[derive(Debug, PartialEq, Encode, Decode)]
struct Indexed(u32, u64);

#[derive(Debug, PartialEq, Encode, Decode)]
struct Struct<A, B, C> {
	pub a: A,
	pub b: B,
	pub c: C,
}


#[derive(Debug, PartialEq, Encode, Decode)]
struct StructWithPhantom {
	pub a: u32,
	pub b: u64,
	_c: ::std::marker::PhantomData<u8>,
}

type TestType = Struct<u32, u64, Vec<u8>>;

impl <A, B, C> Struct<A, B, C> {
	fn new(a: A, b: B, c: C) -> Self {
		Self { a, b, c }
	}
}

#[derive(Debug, PartialEq, Encode, Decode)]
enum EnumType {
	#[codec(index = "15")]
	A,
	B(u32, u64),
	C {
		a: u32,
		b: u64,
	},
}

#[derive(Debug, PartialEq, Encode, Decode)]
enum EnumWithDiscriminant {
	A = 1,
	B = 15,
	C = 255,
}

#[derive(Debug, PartialEq, Encode, Decode)]
struct TestHasCompact<T: HasCompact> {
	#[codec(encoded_as = "<T as HasCompact>::Type")]
	bar: T,
}

#[derive(Debug, PartialEq, Encode, Decode)]
struct TestCompactHasCompact<T: HasCompact> {
	#[codec(compact)]
	bar: T,
}

#[derive(Debug, PartialEq, Encode, Decode)]
enum TestHasCompactEnum<T: HasCompact> {
	Unnamed(#[codec(encoded_as = "<T as HasCompact>::Type")] T),
	Named {
		#[codec(encoded_as = "<T as HasCompact>::Type")]
		bar: T
	},
	UnnamedCompact(#[codec(compact)] T),
	NamedCompact {
		#[codec(compact)]
		bar: T
	},
}

#[derive(Debug, PartialEq, Encode, Decode)]
struct TestCompactAttribute {
	#[codec(compact)]
	bar: u64,
}

#[derive(Debug, PartialEq, Encode, Decode)]
enum TestCompactAttributeEnum {
	Unnamed(#[codec(compact)] u64),
	Named {
		#[codec(compact)]
		bar: u64,
	},
}

#[test]
fn should_work_for_simple_enum() {
	let a = EnumType::A;
	let b = EnumType::B(1, 2);
	let c = EnumType::C { a: 1, b: 2 };

	a.using_encoded(|ref slice| {
		assert_eq!(slice, &b"\x0f");
	});
	b.using_encoded(|ref slice| {
		assert_eq!(slice, &b"\x01\x01\0\0\0\x02\0\0\0\0\0\0\0");
	});
	c.using_encoded(|ref slice| {
		assert_eq!(slice, &b"\x02\x01\0\0\0\x02\0\0\0\0\0\0\0");
	});

	let mut da: &[u8] = b"\x0f";
	assert_eq!(EnumType::decode(&mut da), Some(a));
	let mut db: &[u8] = b"\x01\x01\0\0\0\x02\0\0\0\0\0\0\0";
	assert_eq!(EnumType::decode(&mut db), Some(b));
	let mut dc: &[u8] = b"\x02\x01\0\0\0\x02\0\0\0\0\0\0\0";
	assert_eq!(EnumType::decode(&mut dc), Some(c));
	let mut dz: &[u8] = &[0];
	assert_eq!(EnumType::decode(&mut dz), None);
}

#[test]
fn should_work_for_enum_with_discriminant() {
	EnumWithDiscriminant::A.using_encoded(|ref slice| {
		assert_eq!(slice, &[1]);
	});
	EnumWithDiscriminant::B.using_encoded(|ref slice| {
		assert_eq!(slice, &[15]);
	});
	EnumWithDiscriminant::C.using_encoded(|ref slice| {
		assert_eq!(slice, &[255]);
	});

	let mut da: &[u8] = &[1];
	assert_eq!(EnumWithDiscriminant::decode(&mut da), Some(EnumWithDiscriminant::A));
	let mut db: &[u8] = &[15];
	assert_eq!(EnumWithDiscriminant::decode(&mut db), Some(EnumWithDiscriminant::B));
	let mut dc: &[u8] = &[255];
	assert_eq!(EnumWithDiscriminant::decode(&mut dc), Some(EnumWithDiscriminant::C));
	let mut dz: &[u8] = &[2];
	assert_eq!(EnumWithDiscriminant::decode(&mut dz), None);
}

#[test]
fn should_derive_encode() {
	let v = TestType::new(15, 9, b"Hello world".to_vec());

	v.using_encoded(|ref slice| {
		assert_eq!(slice, &b"\x0f\0\0\0\x09\0\0\0\0\0\0\0\x2cHello world")
	});
}

#[test]
fn should_derive_decode() {
	let slice = b"\x0f\0\0\0\x09\0\0\0\0\0\0\0\x2cHello world".to_vec();

	let v = TestType::decode(&mut &*slice);

	assert_eq!(v, Some(TestType::new(15, 9, b"Hello world".to_vec())));
}

#[test]
fn should_work_for_unit() {
	let v = Unit;

	v.using_encoded(|ref slice| {
		assert_eq!(slice, &[]);
	});

	let mut a: &[u8] = &[];
	assert_eq!(Unit::decode(&mut a), Some(Unit));
}

#[test]
fn should_work_for_indexed() {
	let v = Indexed(1, 2);

	v.using_encoded(|ref slice| {
		assert_eq!(slice, &b"\x01\0\0\0\x02\0\0\0\0\0\0\0")
	});

	let mut v: &[u8] = b"\x01\0\0\0\x02\0\0\0\0\0\0\0";
	assert_eq!(Indexed::decode(&mut v), Some(Indexed(1, 2)));
}

const U64_TEST_COMPACT_VALUES: &[(u64, usize)] = &[
	(0u64, 1usize), (63, 1), (64, 2), (16383, 2),
	(16384, 4), (1073741823, 4),
	(1073741824, 5), (1 << 32 - 1, 5),
	(1 << 32, 6), (1 << 40, 7), (1 << 48, 8), (1 << 56 - 1, 8), (1 << 56, 9), (u64::max_value(), 9)
];

const U64_TEST_COMPACT_VALUES_FOR_ENUM: &[(u64, usize)] = &[
	(0u64, 2usize), (63, 2), (64, 3), (16383, 3),
	(16384, 5), (1073741823, 5),
	(1073741824, 6), (1 << 32 - 1, 6),
	(1 << 32, 7), (1 << 40, 8), (1 << 48, 9), (1 << 56 - 1, 9), (1 << 56, 10), (u64::max_value(), 10)
];

#[test]
fn encoded_as_with_has_compact_works() {
	for &(n, l) in U64_TEST_COMPACT_VALUES {
		let encoded = TestHasCompact { bar: n }.encode();
		println!("{}", n);
		assert_eq!(encoded.len(), l);
		assert_eq!(<TestHasCompact<u64>>::decode(&mut &encoded[..]).unwrap().bar, n);
	}
}

#[test]
fn compact_with_has_compact_works() {
	for &(n, l) in U64_TEST_COMPACT_VALUES {
		let encoded = TestHasCompact { bar: n }.encode();
		println!("{}", n);
		assert_eq!(encoded.len(), l);
		assert_eq!(<TestCompactHasCompact<u64>>::decode(&mut &encoded[..]).unwrap().bar, n);
	}
}

#[test]
fn enum_compact_and_encoded_as_with_has_compact_works() {
	for &(n, l) in U64_TEST_COMPACT_VALUES_FOR_ENUM {
		for value in [
			TestHasCompactEnum::Unnamed(n),
			TestHasCompactEnum::Named { bar: n },
			TestHasCompactEnum::UnnamedCompact(n),
			TestHasCompactEnum::NamedCompact { bar: n },
		].iter() {
			let encoded = value.encode();
			println!("{:?}", value);
			assert_eq!(encoded.len(), l);
			assert_eq!(&<TestHasCompactEnum<u64>>::decode(&mut &encoded[..]).unwrap(), value);
		}
	}
}

#[test]
fn compact_meta_attribute_works() {
	for &(n, l) in U64_TEST_COMPACT_VALUES {
		let encoded = TestCompactAttribute { bar: n }.encode();
		assert_eq!(encoded.len(), l);
		assert_eq!(TestCompactAttribute::decode(&mut &encoded[..]).unwrap().bar, n);
	}
}

#[test]
fn enum_compact_meta_attribute_works() {
	for &(n, l) in U64_TEST_COMPACT_VALUES_FOR_ENUM {
		for value in [ TestCompactAttributeEnum::Unnamed(n), TestCompactAttributeEnum::Named { bar: n } ].iter() {
			let encoded = value.encode();
			assert_eq!(encoded.len(), l);
			assert_eq!(&TestCompactAttributeEnum::decode(&mut &encoded[..]).unwrap(), value);
		}
	}
}

#[test]
fn associated_type_bounds() {
	trait Trait {
		type EncodableType;
		type NonEncodableType;
	}

	#[derive(Encode, Decode, Debug, PartialEq)]
	struct Struct<T: Trait, Type> {
		field: (Vec<T::EncodableType>, Type),
	}

	#[derive(Debug, PartialEq)]
	struct TraitImplementor;

	struct NonEncodableType;

	impl Trait for TraitImplementor {
		type EncodableType = u32;
		type NonEncodableType = NonEncodableType;
	}

	let value: Struct<TraitImplementor, u64> = Struct { field: (vec![1, 2, 3], 42) };
	let encoded = value.encode();
	let decoded: Struct<TraitImplementor, u64> = Struct::decode(&mut &encoded[..]).unwrap();
	assert_eq!(value, decoded);
}

#[test]
fn generic_bound_encoded_as() {
	// This struct does not impl Codec nor HasCompact
	struct StructEncodeAsRef;

	impl From<u32> for StructEncodeAsRef {
		fn from(_: u32) -> Self {
			StructEncodeAsRef
		}
	}

	impl<'a> From<&'a StructEncodeAsRef> for &'a u32 {
		fn from(_: &'a StructEncodeAsRef) -> Self {
			&0
		}
	}

	impl<'a> EncodeAsRef<'a, StructEncodeAsRef> for u32 {
		type RefType = &'a u32;
	}

	#[derive(Debug, PartialEq, Encode, Decode)]
	struct TestGeneric<A: From<u32>>
	where
		u32: for<'a> EncodeAsRef<'a, A>,
	{
		#[codec(encoded_as = "u32")]
		a: A,
	}

	let a = TestGeneric::<StructEncodeAsRef> {
		a: StructEncodeAsRef,
	};
	a.encode();
}

#[test]
fn generic_bound_hascompact() {
	#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
	#[derive(PartialEq, Eq, Clone)]
	// This struct does not impl Codec
	struct StructHasCompact(u8);

	impl CompactAs for StructHasCompact {
		type As = u32;
		fn encode_as(&self) -> &Self::As {
			&0
		}
		fn decode_from(_: Self::As) -> Self {
			StructHasCompact(0)
		}
	}

	impl From<Compact<StructHasCompact>> for StructHasCompact {
		fn from(_: Compact<StructHasCompact>) -> Self {
			StructHasCompact(0)
		}
	}

	#[derive(Debug, PartialEq, Encode, Decode)]
	enum TestGenericHasCompact<T> {
		A {
			#[codec(compact)] a: T
		},
	}

	let a = TestGenericHasCompact::A::<StructHasCompact> {
		a: StructHasCompact(0),
	};

	a.encode();
}

#[test]
fn generic_trait() {
	trait TraitNoCodec {
		type Type;
	}

	struct StructNoCodec;

	#[derive(Debug, PartialEq, Encode, Decode)]
	struct StructCodec;

	impl TraitNoCodec for StructNoCodec {
		type Type = StructCodec;
	}

	#[derive(Debug, PartialEq, Encode, Decode)]
	struct TestGenericTrait<T: TraitNoCodec> {
		t: T::Type,
	}

	let a = TestGenericTrait::<StructNoCodec> {
		t: StructCodec,
	};

	a.encode();
}
