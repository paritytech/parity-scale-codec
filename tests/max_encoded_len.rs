// Copyright (C) 2020-2021 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Tests for MaxEncodedLen derive macro
#![cfg(all(feature = "derive", feature = "max-encoded-len"))]

use jam_codec::{Compact, Decode, Encode, EncodeAsRef, MaxEncodedLen};

#[derive(Encode, MaxEncodedLen)]
struct Primitives {
	bool: bool,
	eight: u8,
}

#[test]
fn primitives_max_length() {
	assert_eq!(Primitives::max_encoded_len(), 2);
}

#[derive(Encode, MaxEncodedLen)]
struct SkippedField {
	bool: bool,
	#[codec(skip)]
	_skipped: u64,
}

#[test]
fn skipped_field_max_length() {
	assert_eq!(SkippedField::max_encoded_len(), 1);
}

#[derive(Encode, MaxEncodedLen)]
struct Composites {
	fixed_size_array: [u8; 128],
	tuple: (u128, u128),
}

#[test]
fn composites_max_length() {
	assert_eq!(Composites::max_encoded_len(), 128 + 16 + 16);
}

#[derive(Encode, MaxEncodedLen)]
struct Generic<T> {
	one: T,
	two: T,
}

#[test]
fn generic_max_length() {
	assert_eq!(Generic::<u8>::max_encoded_len(), u8::max_encoded_len() * 2);
	assert_eq!(Generic::<u32>::max_encoded_len(), u32::max_encoded_len() * 2);
}

#[derive(Encode, MaxEncodedLen)]
struct TwoGenerics<T, U> {
	t: T,
	u: U,
}

#[test]
fn two_generics_max_length() {
	assert_eq!(
		TwoGenerics::<u8, u16>::max_encoded_len(),
		u8::max_encoded_len() + u16::max_encoded_len()
	);
	assert_eq!(
		TwoGenerics::<Compact<u64>, [u16; 8]>::max_encoded_len(),
		Compact::<u64>::max_encoded_len() + <[u16; 8]>::max_encoded_len()
	);
}

#[derive(Encode, MaxEncodedLen)]
struct UnitStruct;

#[test]
fn unit_struct_max_length() {
	assert_eq!(UnitStruct::max_encoded_len(), 0);
}

#[derive(Encode, MaxEncodedLen)]
struct TupleStruct(u8, u32);

#[test]
fn tuple_struct_max_length() {
	assert_eq!(TupleStruct::max_encoded_len(), u8::max_encoded_len() + u32::max_encoded_len());
}

#[derive(Encode, MaxEncodedLen)]
struct TupleGeneric<T>(T, T);

#[test]
fn tuple_generic_max_length() {
	assert_eq!(TupleGeneric::<u8>::max_encoded_len(), u8::max_encoded_len() * 2);
	assert_eq!(TupleGeneric::<u32>::max_encoded_len(), u32::max_encoded_len() * 2);
}

#[derive(Encode)]
struct ConstU32<const N: u32>;

trait Get<T>: Encode {
	fn get() -> T;
}

impl<const N: u32> Get<u32> for ConstU32<N> {
	fn get() -> u32 {
		N
	}
}

#[derive(Encode)]
struct SomeVec<T, N> {
	element: T,
	size: N,
}

impl<T: MaxEncodedLen, N: Get<u32>> MaxEncodedLen for SomeVec<T, N> {
	fn max_encoded_len() -> usize {
		T::max_encoded_len() * N::get() as usize
	}
}

#[derive(Encode, MaxEncodedLen)]
#[codec(mel_bound(N: Get<u32>))]
struct SizeGeneric<N> {
	vec: SomeVec<u64, N>,
}

#[test]
fn some_vec_max_length() {
	assert_eq!(SomeVec::<u64, ConstU32<3>>::max_encoded_len(), u64::max_encoded_len() * 3);
	assert_eq!(SizeGeneric::<ConstU32<5>>::max_encoded_len(), u64::max_encoded_len() * 5);
}

#[derive(Encode, MaxEncodedLen)]
#[allow(unused)]
enum UnitEnum {
	A,
	B,
}

#[test]
fn unit_enum_max_length() {
	assert_eq!(UnitEnum::max_encoded_len(), 1);
}

#[derive(Encode, MaxEncodedLen)]
#[allow(unused)]
enum TupleEnum {
	A(u32),
	B,
}

#[test]
fn tuple_enum_max_length() {
	assert_eq!(TupleEnum::max_encoded_len(), 1 + u32::max_encoded_len());
}

#[derive(Encode, MaxEncodedLen)]
#[allow(unused)]
enum StructEnum {
	A { sixty_four: u64, one_twenty_eight: u128 },
	B,
}

#[test]
fn struct_enum_max_length() {
	assert_eq!(StructEnum::max_encoded_len(), 1 + u64::max_encoded_len() + u128::max_encoded_len());
}

// ensure that enums take the max of variant length, not the sum
#[derive(Encode, MaxEncodedLen)]
#[allow(unused)]
enum EnumMaxNotSum {
	A(u32),
	B(u32),
}

#[test]
fn enum_max_not_sum_max_length() {
	assert_eq!(EnumMaxNotSum::max_encoded_len(), 1 + u32::max_encoded_len());
}

#[derive(Encode, MaxEncodedLen)]
struct CompactField {
	#[codec(compact)]
	a: u32,
	b: u32,
}

#[test]
fn compact_field_max_length() {
	assert_eq!(
		CompactField::max_encoded_len(),
		Compact::<u32>::max_encoded_len() + u32::max_encoded_len()
	);
}

#[derive(Encode, MaxEncodedLen)]
#[allow(unused)]
enum CompactVariant {
	A(u8),
	B(#[codec(compact)] u32),
}

#[test]
fn compact_variant_max_length() {
	assert_eq!(CompactVariant::max_encoded_len(), 1 + Compact::<u32>::max_encoded_len());
}

#[derive(Encode, MaxEncodedLen)]
struct U64(u64);

impl<'a> EncodeAsRef<'a, u32> for U64 {
	// Obviously broken but will do for this test
	type RefType = &'a u32;
}

#[derive(Encode, MaxEncodedLen)]
struct EncodedAsField {
	a: u32,
	#[codec(encoded_as = "U64")]
	b: u32,
}

#[test]
fn encoded_as_field_max_length() {
	assert_eq!(EncodedAsField::max_encoded_len(), u32::max_encoded_len() + U64::max_encoded_len());
}

#[derive(Encode, MaxEncodedLen)]
#[allow(unused)]
enum EncodedAsVariant {
	A(#[codec(encoded_as = "U64")] u32),
	B(u32),
}

#[test]
fn encoded_as_variant_max_length() {
	assert_eq!(EncodedAsVariant::max_encoded_len(), 1 + U64::max_encoded_len());
}

#[test]
fn skip_type_params() {
	#[derive(Encode, Decode, MaxEncodedLen)]
	#[codec(mel_bound(skip_type_params(N)))]
	struct SomeData<T, N: SomeTrait> {
		element: T,
		size: std::marker::PhantomData<N>,
	}

	trait SomeTrait {}

	struct SomeStruct;

	impl SomeTrait for SomeStruct {}

	assert_eq!(SomeData::<u32, SomeStruct>::max_encoded_len(), 4);
}

#[test]
fn skip_enum_struct_test() {
	#[derive(Default)]
	struct NoCodecType;

	struct NoCodecNoDefaultType;

	#[derive(Encode, Decode, MaxEncodedLen)]
	enum Enum<T = NoCodecType, S = NoCodecNoDefaultType> {
		#[codec(skip)]
		A(S),
		B {
			#[codec(skip)]
			_b1: T,
			b2: u32,
		},
		C(#[codec(skip)] T, u32),
	}

	#[derive(Encode, Decode, MaxEncodedLen)]
	struct StructNamed<T = NoCodecType> {
		#[codec(skip)]
		a: T,
		b: u32,
	}

	#[derive(Encode, Decode, MaxEncodedLen)]
	struct StructUnnamed<T = NoCodecType>(#[codec(skip)] T, u32);

	assert_eq!(Enum::<NoCodecType, NoCodecNoDefaultType>::max_encoded_len(), 5);
	assert_eq!(StructNamed::<NoCodecType>::max_encoded_len(), 4);
	assert_eq!(StructUnnamed::<NoCodecType>::max_encoded_len(), 4);

	// Use the fields to avoid unused warnings.
	let _ = Enum::<NoCodecType, NoCodecNoDefaultType>::A(NoCodecNoDefaultType);
	let _ = StructNamed::<NoCodecType> { a: NoCodecType, b: 0 }.a;
}
