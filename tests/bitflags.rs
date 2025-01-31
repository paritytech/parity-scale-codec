use parity_scale_codec_derive::{Decode, Encode};

#[enumflags2::bitflags]
#[repr(u64)]
#[derive(Copy, Clone, Encode, Decode)]
pub enum EnumWithU64Repr {
	Variant1,
	Variant2,
	Variant3,
	Variant4,
}
