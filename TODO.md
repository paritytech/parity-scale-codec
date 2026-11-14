# TODO

Places where jam-codec does not follow the Gray Paper codec (serialization.tex, "Common Terms").
The GP defines decode as the inverse of encode, so a decoder must reject all octets that the
encoder never writes.

## `BTreeMap` and `BTreeSet` decode accept keys out of order and repeated keys

src/codec.rs:1273-1282 (`BTreeMap`), src/codec.rs:1295-1304 (`BTreeSet`)

The encode writes the keys in ascending order, as the GP requires. The decode does not compare a
key with the key before it, and for a repeated key it keeps the last value.

- `BTreeMap<u8, u8>`: `02 02 00 01 00` decodes to `{1: 0, 2: 0}`. The GP encoding is `02 01 00 02 00`.
- `BTreeMap<u8, u8>`: `02 01 00 01 05` decodes to `{1: 5}`. The GP encoding is `01 01 05`.
- `BTreeSet<u8>`: `02 09 07` decodes to `{7, 9}`. The GP encoding is `02 07 09`.
- `BTreeSet<u8>`: `02 07 07` decodes to `{7}`. The GP encoding is `01 07`.

Fix: reject a key that is not strictly greater than the key before it.

## `BitVec` decode does not check the padding bits

src/bit_vec.rs:54-74

The GP writes 0 in the bits after the last bit. The decode removes these bits (src/bit_vec.rs:72)
and does not check them. `01 03` decodes to `[1]`. The GP encoding is `01 01`.

Only `BitVec<u8, Lsb0>` has the GP layout. `Msb0` and stores wider than `u8` do not.

Fix: reject the input if a bit after position `bits` is 1.

## Enum indexes of 128 or more are not GP naturals

derive/src/encode.rs:335, 368, 386; derive/src/decode.rs:63, 88; derive/src/utils.rs:72

The GP writes a discriminator as a natural. The derive writes the variant index as one octet and
accepts indexes up to 255. For index 128, jam-codec writes `80` and the GP writes `80 80`. The GP
discriminators are 0 to 6 today, so no GP type is affected.

Fix: limit `#[codec(index)]` to 127 in the check at derive/src/utils.rs:72.
