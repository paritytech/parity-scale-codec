use parity_scale_codec_derive::{Encode, Decode};

#[enumflags2::bitflags]
#[repr(u64)]
#[derive(Copy, Clone, Encode, Decode)]
pub enum EnumWithU64Repr {
	Variant1,
	Variant2,
	Variant3,
	Variant4,
}
