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

use parity_scale_codec::{
	Compact, CompactAs, Decode, Encode, EncodeAsRef, Error, HasCompact, Output,
};
#[cfg(not(feature = "derive"))]
use parity_scale_codec_derive::{Decode, Encode};
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Encode, Decode)]
struct Unit;

#[derive(Debug, PartialEq, Encode, Decode)]
struct Indexed(u32, u64);

#[derive(Debug, PartialEq, Encode, Decode, Default)]
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

impl<A, B, C> Struct<A, B, C> {
	fn new(a: A, b: B, c: C) -> Self {
		Self { a, b, c }
	}
}

#[derive(Debug, PartialEq, Encode, Decode)]
enum EnumType {
	#[codec(index = 15)]
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
		bar: T,
	},
	UnnamedCompact(#[codec(compact)] T),
	NamedCompact {
		#[codec(compact)]
		bar: T,
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
	assert_eq!(EnumType::decode(&mut da).ok(), Some(a));
	let mut db: &[u8] = b"\x01\x01\0\0\0\x02\0\0\0\0\0\0\0";
	assert_eq!(EnumType::decode(&mut db).ok(), Some(b));
	let mut dc: &[u8] = b"\x02\x01\0\0\0\x02\0\0\0\0\0\0\0";
	assert_eq!(EnumType::decode(&mut dc).ok(), Some(c));
	let mut dz: &[u8] = &[0];
	assert_eq!(EnumType::decode(&mut dz).ok(), None);
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
	assert_eq!(EnumWithDiscriminant::decode(&mut da), Ok(EnumWithDiscriminant::A));
	let mut db: &[u8] = &[15];
	assert_eq!(EnumWithDiscriminant::decode(&mut db), Ok(EnumWithDiscriminant::B));
	let mut dc: &[u8] = &[255];
	assert_eq!(EnumWithDiscriminant::decode(&mut dc), Ok(EnumWithDiscriminant::C));
	let mut dz: &[u8] = &[2];
	assert_eq!(EnumWithDiscriminant::decode(&mut dz).ok(), None);
}

#[test]
fn should_derive_encode() {
	let v = TestType::new(15, 9, b"Hello world".to_vec());

	v.using_encoded(|ref slice| assert_eq!(slice, &b"\x0f\0\0\0\x09\0\0\0\0\0\0\0\x2cHello world"));
}

#[test]
fn should_derive_decode() {
	let slice = b"\x0f\0\0\0\x09\0\0\0\0\0\0\0\x2cHello world".to_vec();

	let v = TestType::decode(&mut &*slice);

	assert_eq!(v, Ok(TestType::new(15, 9, b"Hello world".to_vec())));
}

#[test]
fn should_work_for_unit() {
	let v = Unit;

	v.using_encoded(|ref slice| {
		assert_eq!(slice, &[]);
	});

	let mut a: &[u8] = &[];
	assert_eq!(Unit::decode(&mut a), Ok(Unit));
}

#[test]
fn should_work_for_indexed() {
	let v = Indexed(1, 2);

	v.using_encoded(|ref slice| assert_eq!(slice, &b"\x01\0\0\0\x02\0\0\0\0\0\0\0"));

	let mut v: &[u8] = b"\x01\0\0\0\x02\0\0\0\0\0\0\0";
	assert_eq!(Indexed::decode(&mut v), Ok(Indexed(1, 2)));
}

#[test]
#[should_panic(expected = "Not enough data to fill buffer")]
fn correct_error_for_indexed_0() {
	let mut wrong: &[u8] = b"\x08";
	Indexed::decode(&mut wrong).unwrap();
}

#[test]
#[should_panic(expected = "Not enough data to fill buffer")]
fn correct_error_for_indexed_1() {
	let mut wrong: &[u8] = b"\0\0\0\0\x01";
	Indexed::decode(&mut wrong).unwrap();
}

#[test]
#[should_panic(expected = "Not enough data to fill buffer")]
fn correct_error_for_enumtype() {
	let mut wrong: &[u8] = b"\x01";
	EnumType::decode(&mut wrong).unwrap();
}

#[test]
#[should_panic(expected = "Not enough data to fill buffer")]
fn correct_error_for_named_struct_1() {
	let mut wrong: &[u8] = b"\x01";
	Struct::<u32, u32, u32>::decode(&mut wrong).unwrap();
}

#[test]
#[should_panic(expected = "Not enough data to fill buffer")]
fn correct_error_for_named_struct_2() {
	let mut wrong: &[u8] = b"\0\0\0\0\x01";
	Struct::<u32, u32, u32>::decode(&mut wrong).unwrap();
}

const U64_TEST_COMPACT_VALUES: &[(u64, usize)] = &[
	(0u64, 1usize),
	(63, 1),
	(64, 2),
	(16383, 2),
	(16384, 4),
	(1073741823, 4),
	(1073741824, 5),
	(1 << 32 - 1, 5),
	(1 << 32, 6),
	(1 << 40, 7),
	(1 << 48, 8),
	(1 << 56 - 1, 8),
	(1 << 56, 9),
	(u64::max_value(), 9),
];

const U64_TEST_COMPACT_VALUES_FOR_ENUM: &[(u64, usize)] = &[
	(0u64, 2usize),
	(63, 2),
	(64, 3),
	(16383, 3),
	(16384, 5),
	(1073741823, 5),
	(1073741824, 6),
	(1 << 32 - 1, 6),
	(1 << 32, 7),
	(1 << 40, 8),
	(1 << 48, 9),
	(1 << 56 - 1, 9),
	(1 << 56, 10),
	(u64::max_value(), 10),
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
		]
		.iter()
		{
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
		for value in
			[TestCompactAttributeEnum::Unnamed(n), TestCompactAttributeEnum::Named { bar: n }]
				.iter()
		{
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

	let a = TestGeneric::<StructEncodeAsRef> { a: StructEncodeAsRef };
	a.encode();
}

#[test]
fn generic_bound_hascompact() {
	#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
	#[derive(PartialEq, Eq, Clone)]
	// This struct does not impl Codec
	struct StructHasCompact(u32);

	impl CompactAs for StructHasCompact {
		type As = u32;
		fn encode_as(&self) -> &Self::As {
			&0
		}
		fn decode_from(_: Self::As) -> Result<Self, Error> {
			Ok(StructHasCompact(0))
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
			#[codec(compact)]
			a: T,
		},
	}

	let a = TestGenericHasCompact::A::<StructHasCompact> { a: StructHasCompact(0) };

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

	let a = TestGenericTrait::<StructNoCodec> { t: StructCodec };

	a.encode();
}

#[test]
fn recursive_variant_1_encode_works() {
	#[derive(Debug, PartialEq, Encode, Decode, Default)]
	struct Recursive<N> {
		data: N,
		other: Vec<Recursive<N>>,
	}

	let val: Recursive<u32> = Recursive::default();
	val.encode();
}

#[test]
fn recursive_variant_2_encode_works() {
	#[derive(Debug, PartialEq, Encode, Decode, Default)]
	struct Recursive<A, B, N> {
		data: N,
		other: Vec<Struct<A, B, Recursive<A, B, N>>>,
	}

	let val: Recursive<u32, i32, u32> = Recursive::default();
	val.encode();
}

#[test]
fn private_type_in_where_bound() {
	// Make the `private type `private_type_in_where_bound::Private` in public interface` warning
	// an error.
	#![deny(warnings)]

	#[derive(Debug, PartialEq, Encode, Decode, Default)]
	struct Private;

	#[derive(Debug, PartialEq, Encode, Decode, Default)]
	#[codec(dumb_trait_bound)]
	pub struct Test<N> {
		data: Vec<(N, Private)>,
	}

	let val: Test<u32> = Test::default();
	val.encode();
}

#[test]
fn encode_decode_empty_enum() {
	#[derive(Encode, Decode, PartialEq, Debug)]
	enum EmptyEnumDerive {}

	fn impls_encode_decode<T: Encode + Decode>() {}
	impls_encode_decode::<EmptyEnumDerive>();

	assert_eq!(
		EmptyEnumDerive::decode(&mut &[1, 2, 3][..]),
		Err("Could not decode `EmptyEnumDerive`, variant doesn't exist".into())
	);
}

#[test]
fn codec_vec_u8() {
	for v in [vec![0u8; 0], vec![0u8; 10], vec![0u8; 100], vec![0u8; 1000]].iter() {
		let e = v.encode();
		assert_eq!(v, &Vec::<u8>::decode(&mut &e[..]).unwrap());
	}
}

#[test]
fn recursive_type() {
	#[derive(Encode, Decode)]
	pub enum Foo {
		T(Box<Bar>),
		A,
	}

	#[derive(Encode, Decode)]
	pub struct Bar {
		field: Foo,
	}
}

#[test]
fn crafted_input_for_vec_u8() {
	assert_eq!(
		Vec::<u8>::decode(&mut &Compact(u32::max_value()).encode()[..])
			.err()
			.unwrap()
			.to_string(),
		"Not enough data to decode vector",
	);
}

#[test]
fn crafted_input_for_vec_t() {
	let msg: String = if cfg!(target_endian = "big") {
		// use unoptimize decode
		"Not enough data to fill buffer".into()
	} else {
		"Not enough data to decode vector".into()
	};

	assert_eq!(
		Vec::<u32>::decode(&mut &Compact(u32::max_value()).encode()[..])
			.err()
			.unwrap()
			.to_string(),
		msg,
	);
}

#[test]
fn weird_derive() {
	// Tests that compilation succeeds when the macro invocation
	// hygiene context is different from the field hygiene context.
	macro_rules! make_struct {
		(#[$attr:meta]) => {
			#[$attr]
			pub struct MyStruct {
				field: u8,
			}
		};
	}

	make_struct!(#[derive(Encode, Decode)]);
}

#[test]
fn output_trait_object() {
	let _: Box<dyn Output>;
}

#[test]
fn custom_trait_bound() {
	#[derive(Encode, Decode)]
	#[codec(encode_bound(N: Encode, T: Default))]
	#[codec(decode_bound(N: Decode, T: Default))]
	struct Something<T, N> {
		hello: Hello<T>,
		val: N,
	}

	#[derive(Encode, Decode)]
	#[codec(encode_bound())]
	#[codec(decode_bound())]
	struct Hello<T> {
		_phantom: std::marker::PhantomData<T>,
	}

	#[derive(Default)]
	struct NotEncode;

	let encoded =
		Something::<NotEncode, u32> { hello: Hello { _phantom: Default::default() }, val: 32u32 }
			.encode();

	Something::<NotEncode, u32>::decode(&mut &encoded[..]).unwrap();
}

#[test]
#[cfg(feature = "bit-vec")]
fn bit_vec_works() {
	use bitvec::prelude::*;
	use parity_scale_codec::DecodeAll;

	// Try some fancy stuff
	let original_vec = bitvec![u8, Msb0; 1; 8];
	let mut original_vec_clone = original_vec.clone();
	original_vec_clone = original_vec_clone.split_off(5);
	original_vec_clone.push(true);
	original_vec_clone.push(true);
	original_vec_clone.push(true);
	original_vec_clone.push(true);
	original_vec_clone.push(true);

	assert_eq!(original_vec, original_vec_clone);

	#[derive(Decode, Encode, PartialEq, Debug)]
	struct MyStruct {
		v: BitVec<u8, Msb0>,
		x: u8,
	}

	let v1 = MyStruct { v: original_vec, x: 42 }.encode();
	let v2 = MyStruct { v: original_vec_clone, x: 42 }.encode();
	assert_eq!(v1, v2);

	let v1 = MyStruct::decode(&mut &v1[..]).unwrap();
	let v2 = MyStruct::decode_all(&mut &v2[..]).unwrap();
	assert_eq!(v1.x, v2.x);
}

#[test]
fn no_warning_for_deprecated() {
	#[derive(Encode, Decode)]
	pub enum MyEnum {
		VariantA,
		#[deprecated]
		VariantB,
	}
}
