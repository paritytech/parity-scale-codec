use jam_codec::{Encode, EncodeAsRef, MaxEncodedLen};

#[derive(Encode)]
struct NotMel;

impl<'a> EncodeAsRef<'a, u32> for NotMel {
	// Obviously broken but will do for this test
	type RefType = &'a u32;
}

#[derive(Encode, MaxEncodedLen)]
struct UnsupportedEncodedAsField {
	mel: u32,
	#[codec(encoded_as = "NotMel")]
	not_mel: u32,
}

fn main() {}
