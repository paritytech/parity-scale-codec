use jam_codec::{Decode, DecodeWithMemTracking};

#[derive(Decode)]
struct Base {}

#[derive(Decode, DecodeWithMemTracking)]
struct Wrapper {
	base: Base,
}

fn main() {}
