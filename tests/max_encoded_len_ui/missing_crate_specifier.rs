use jam_codec::{Encode, MaxEncodedLen};

#[derive(Encode, MaxEncodedLen)]
#[codec(jam_codec)]
struct Example;

fn main() {
	let _ = Example::max_encoded_len();
}
