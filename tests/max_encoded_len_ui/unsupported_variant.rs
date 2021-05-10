use parity_scale_codec::{MaxEncodedLen, Encode};

#[derive(Encode)]
struct NotMel;

#[derive(Encode, MaxEncodedLen)]
enum UnsupportedVariant {
	NotMel(NotMel),
}

fn main() {}
