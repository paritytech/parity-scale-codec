# TODO

Places where jam-codec does not follow the Gray Paper codec (serialization.tex, "Common Terms").
The GP defines decode as the inverse of encode, so a decoder must reject all octets that the
encoder never writes.

## Enum indexes of 128 or more are not GP naturals

derive/src/encode.rs:335, 368, 386; derive/src/decode.rs:63, 88; derive/src/utils.rs:72

The GP writes a discriminator as a natural. The derive writes the variant index as one octet and
accepts indexes up to 255. For index 128, jam-codec writes `80` and the GP writes `80 80`. The GP
discriminators are 0 to 6 today, so no GP type is affected.

Fix: limit `#[codec(index)]` to 127 in the check at derive/src/utils.rs:72.
