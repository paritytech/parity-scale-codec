use jam_codec::{Encode, MaxEncodedLen};

#[derive(Encode, MaxEncodedLen)]
union Union {
	a: u8,
	b: u16,
}

fn main() {}
