#[macro_use]
extern crate parity_codec_derive;

use parity_codec::{Encode, HasCompact};
#[cfg(test)]
use parity_codec::Decode;

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
	let s = S { x };
	let sc = Sc { x };
	let sh = Sh { x };
	let u = U(x);
	let uc = Uc(x);
	let uh = Uh(x);

	let mut s_encoded: &[u8] = &[3, 0, 0, 0];
	let mut sc_encoded: &[u8] = &[12];
	let mut sh_encoded: &[u8] = &[12];
	let mut u_encoded: &[u8] = &[3, 0, 0, 0];
	let mut uc_encoded: &[u8] = &[12];
	let mut uh_encoded: &[u8] = &[12];

	assert_eq!(s.encode(), s_encoded);
	assert_eq!(sc.encode(), sc_encoded);
	assert_eq!(sh.encode(), sh_encoded);
	assert_eq!(u.encode(), u_encoded);
	assert_eq!(uc.encode(), uc_encoded);
	assert_eq!(uh.encode(), uh_encoded);

	assert_eq!(s, S::decode(&mut s_encoded).unwrap());
	assert_eq!(sc, Sc::decode(&mut sc_encoded).unwrap());
	assert_eq!(sh, Sh::decode(&mut sh_encoded).unwrap());
	assert_eq!(u, U::decode(&mut u_encoded).unwrap());
	assert_eq!(uc, Uc::decode(&mut uc_encoded).unwrap());
	assert_eq!(uh, Uh::decode(&mut uh_encoded).unwrap());
}
