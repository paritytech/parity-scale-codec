#[derive(::parity_scale_codec::Encode)]
#[codec(crate = ::parity_scale_codec)]
pub enum Enum {
    #[codec(index = MY_CONST_INDEX)]
    Variant1,
    #[codec(index = MY_CONST_INDEX)]
    Variant2,
}

const MY_CONST_INDEX: u8 = 1;

fn main() {}
