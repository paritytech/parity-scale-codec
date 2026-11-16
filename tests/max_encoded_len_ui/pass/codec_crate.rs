//! This test case demonstrates correct use of the `#[codec(crate = path)]` attribute.

use jam_codec::{self as codec, Encode, MaxEncodedLen};

#[derive(Encode, MaxEncodedLen)]
#[codec(crate = codec)]
struct Example;

fn main() {
	let _ = Example::max_encoded_len();
}
