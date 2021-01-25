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

#[cfg(not(feature="derive"))]
use parity_scale_codec_derive::Decode;
use parity_scale_codec::Decode;

#[derive(Decode, Debug)]
struct Wrapper<T>(T);

#[derive(Decode, Debug)]
struct StructNamed {
	foo: u16
}

#[derive(Decode, Debug)]
struct StructUnnamed(u16);

#[derive(Decode, Debug)]
enum E {
	VariantNamed { foo: u16, },
	VariantUnnamed(u16),
}

#[test]
fn full_error_struct_named() {
	let encoded = vec![0];
	let err = r#"Could not decode `Wrapper.0`:
	Could not decode `StructNamed::foo`:
		Not enough data to fill buffer
"#;

	assert_eq!(
		Wrapper::<StructNamed>::decode(&mut &encoded[..]).unwrap_err().to_string(),
		String::from(err),
	);
}

#[test]
fn full_error_struct_unnamed() {
	let encoded = vec![0];
	let err = r#"Could not decode `Wrapper.0`:
	Could not decode `StructUnnamed.0`:
		Not enough data to fill buffer
"#;

	assert_eq!(
		Wrapper::<StructUnnamed>::decode(&mut &encoded[..]).unwrap_err().to_string(),
		String::from(err),
	);
}

#[test]
fn full_error_enum_unknown_variant() {
	let encoded = vec![2];
	let err = r#"Could not decode `E`, variant doesn't exist"#;

	assert_eq!(
		E::decode(&mut &encoded[..]).unwrap_err().to_string(),
		String::from(err),
	);
}

#[test]
fn full_error_enum_named_field() {
	let encoded = vec![0, 0];
	let err = r#"Could not decode `E::VariantNamed::foo`:
	Not enough data to fill buffer
"#;

	assert_eq!(
		E::decode(&mut &encoded[..]).unwrap_err().to_string(),
		String::from(err),
	);
}

#[test]
fn full_error_enum_unnamed_field() {
	let encoded = vec![1, 0];
	let err = r#"Could not decode `E::VariantUnnamed.0`:
	Not enough data to fill buffer
"#;

	assert_eq!(
		E::decode(&mut &encoded[..]).unwrap_err().to_string(),
		String::from(err),
	);
}
