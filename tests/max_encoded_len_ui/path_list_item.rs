use parity_scale_codec::{Encode, MaxEncodedLen};

#[derive(Encode, MaxEncodedLen)]
#[codec(frame_support::max_encoded_len)]
struct Example;

fn main() {
	let _ = Example::max_encoded_len();
}
