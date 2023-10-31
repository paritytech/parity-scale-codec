---
title: "Specification"
weight: 5
# bookFlatSection: false
bookToc: false
# bookHidden: false
# bookCollapseSection: false
# bookComments: false
# bookSearchExclude: false
math: true
---

# Specification

SCALE defines encodings for native Rust types, and constructs encodings for composite types, such as structs, by concatenating the encodings of their constituents – that is, the elementary types that form these respective complex types. Additionally, some variable-length types are encoded with their length prefixed. In this way, the encoding of any type can be simplified to the concatenation of encodings of less complex types.

This table offers a concise overview of the SCALE codec with examples. For more detailed, hands-on explanations, please refer to the [encode section]({{< ref "/encode" >}}).

| Data type | Encoding Description |  SCALE decoded value	| SCALE encoded value |
| --        | --          | -- | -- |
| Unit | Encoded as an empty byte array. | `()` | `[]` |
| Boolean    | Encoded using the least significant bit of a single byte. | `true` | `[01]` |
|           |                                                           | `false`| `[00]` |
| Integer | By default integers are encoded using a fixed-width little-endian format. | `69i8` | `[2a]` |
|         |                                                                      | `69u32`| `[45, 00, 00, 00]`|
|         | Unsigned integers $n$ also have a compact encoding. There are four modes. | | | |
|         | Single-byte mode: Upper six bits are the LE encoding of the value. For $0 \leq n \leq 2^6 - 1$. |`0u8` | `[00]` |
|         | Two-byte mode: Upper six bits and the following byte is the LE encoding of the value. For $2^6 \leq n \leq 2^{14} - 1$. |`69u8` | `[15, 01]` | 
|         | Four-byte mode: Upper six bits and the following three bytes are the LE encoding of the value. For $2^{14} \leq n \leq 2^{30} - 1$. |`65535u32` | `[fe, ff, 03, 00]` |
|         | Big-integer mode: The upper six bits are the number of bytes following, minus four. The value is contained, LE encoded, in the bytes following. The final (most significant) byte must be non-zero. For $2^{30} \leq n \leq 2^{536} - 1$. |`1073741824u64` | `[03, 00, 00, 00, 40]` |
| Vector | Encoded by concatening the encodings of its items and prefixing with the compactly encoded length of the vector. |`vec![1u8, 2u8, 4u8]` | `[0c, 01, 02, 04]` |
| String | Encoded as `Vec<u8>` with UTF-8 characters. | `"SCALE♡"` | `[20, 53, 43, 41, 4c, 45, e2, 99, a1]` |
| Tuple, Struct, Array | Encoded by concatenating the encodings of their respective elements consecutively. |`(1u8, true, "OK")` | `[01, 01, 08, 4f, 4b]` |
| | | `MyStruct{id: 1u8, is_val: true, msg: "OK"}`| `[01, 01, 08, 4f, 4b]` |
| | |`[64u16, 512u16]` | `[40, 00, 00, 02]` |
| Enum | Encoded by the `u8`-index of the respective variant, followed by the encoded value if it is present. | `Example::Second(8u16)` | `[01, 08, 00]`|
| Result | Encoded by prefixing the encoded inner value with `0x00` if the operation was successful, and `0x01` if the operation was unsuccessful. |`Ok::<u32, ()>(42u32)` | `[00, 2a, 00, 00, 00]` |
| |  |`Err::<u32, ()>(())` | `[01]` |
| Option | Encoded by prefixing the inner encoded value of `Some` with `0x01` and encoding `None` as `0x00`. |`Some(69u8)` | `[01, 45]` |
|  |  | `None::<u8>` | `[00]` |