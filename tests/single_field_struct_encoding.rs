#[macro_use]
extern crate parity_codec_derive;

use parity_codec::{Encode, Decode, HasCompact, Compact, EncodeAsRef, CompactAs};

#[derive(Debug, PartialEq, Encode, Decode)]
struct S {
	x: u32,
}

#[derive(Debug, PartialEq, Encode, Decode)]
struct Sc {
	#[codec(compact)]
	x: u32,
}

#[derive(Debug, PartialEq, Encode, Decode)]
struct Sh<T: HasCompact> {
	#[codec(encoded_as = "<T as HasCompact>::Type")]
	x: T,
}

#[derive(Debug, PartialEq, Encode, Decode)]
struct U(u32);

#[derive(Debug, PartialEq, Encode, Decode)]
struct Uc(#[codec(compact)] u32);

#[derive(Debug, PartialEq, Encode, Decode)]
struct Uh<T: HasCompact>(#[codec(encoded_as = "<T as HasCompact>::Type")] T);

#[test]
fn test_encoding() {
	let x = 3u32;
	let s = S { x }.encode();
	let sc = Sc { x }.encode();
	let sh = Sh { x }.encode();
	let u = U(x).encode();
	let uc = Uc(x).encode();
	let uh = Uh(x).encode();

	assert_eq!(&s, &[3, 0, 0, 0]);
	assert_eq!(&sc, &[12]);
	assert_eq!(&sh, &[12]);
	assert_eq!(&u, &[3, 0, 0, 0]);
	assert_eq!(&uc, &[12]);
	assert_eq!(&uh, &[12]);
}
