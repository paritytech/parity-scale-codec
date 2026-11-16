#[derive(::jam_codec::Decode, ::jam_codec::Encode)]
#[codec(crate = ::jam_codec)]
enum T {
	A = 1,
	B,
}

#[derive(::jam_codec::Decode, ::jam_codec::Encode)]
#[codec(crate = ::jam_codec)]
enum T2 {
	#[codec(index = 1)]
	A,
	B,
}

fn main() {}
