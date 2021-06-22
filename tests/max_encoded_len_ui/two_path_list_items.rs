use parity_scale_codec::{Encode, MaxEncodedLen};

#[derive(Encode, MaxEncodedLen)]
#[max_encoded_len_mod(max_encoded_len, parity_scale_codec::max_encoded_len)]
struct Example;

fn main() {
	let _ = Example::max_encoded_len();
}
