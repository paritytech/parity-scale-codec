use parity_scale_codec::{MaxEncodedLen, Encode};

#[derive(Encode, MaxEncodedLen)]
union Union {
	a: u8,
	b: u16,
}

fn main() {}
