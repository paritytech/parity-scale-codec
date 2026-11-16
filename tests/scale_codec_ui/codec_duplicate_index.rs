#[derive(::jam_codec::Decode, ::jam_codec::Encode)]
#[codec(crate = ::jam_codec)]
enum T {
	A = 3,
	#[codec(index = 3)]
	B,
}

#[derive(::jam_codec::Decode, ::jam_codec::Encode)]
#[codec(crate = ::jam_codec)]
enum T1 {
	A,
	#[codec(index = 0)]
	B,
}

fn main() {}
