use jam_codec::{Encode, MaxEncodedLen};

#[derive(Encode)]
struct NotMel;

#[derive(Encode, MaxEncodedLen)]
struct UnsupportedField {
	mel: u32,
	not_mel: NotMel,
}

fn main() {}
