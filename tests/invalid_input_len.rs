use parity_scale_codec::{Encode, Decode, OptionBool, Compact};

// Test that not all input is decoded.
#[test]
fn vec_of_vec() {
	let mut input = Compact::<u32>(16).encode();
	input.extend(&[0; 15]);
	let input = &mut &input[..];

	let input_start_len = input.len();
	let msg = <Vec<Vec<()>>>::decode(input).unwrap_err().what();
	assert_eq!(msg, "Not enough data for required minimum length");

	// Decoding stopped early, before vec allocation.
	assert_eq!(input.len(), input_start_len - 1);
}

// Test error returned.
#[test]
#[should_panic(expected = "Not enough data to fill buffer")]
fn option_bool() {
	let input: [u8; 0] = [];
	OptionBool::decode(&mut &input[..]).unwrap();
}
