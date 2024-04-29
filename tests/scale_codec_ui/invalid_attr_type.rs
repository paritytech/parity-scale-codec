#[derive(::parity_scale_codec::Encode)]
#[codec(crate = ::parity_scale_codec)]
pub enum Enum {
    #[codec(index = "invalid")]
    Variant1,
}

fn main() {}
