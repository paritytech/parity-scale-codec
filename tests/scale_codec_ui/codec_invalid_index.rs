#[derive(::jam_codec::Decode, ::jam_codec::Encode)]
#[codec(crate = ::jam_codec)]
enum T {
	A = 3,
	#[codec(index = 524)]
	B,
}

fn main() {}
