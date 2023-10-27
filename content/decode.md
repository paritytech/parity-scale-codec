---
title: "Decode"
weight: 3
# bookFlatSection: false
# bookToc: true
# bookHidden: false
# bookCollapseSection: false
# bookComments: false
# bookSearchExclude: false
math: true
---


# 1. Decoding
Since SCALE is non-descriptive, the proper metadata is needed to decode raw bytes into the appropriate types.

```rust
use parity_scale_codec::{ Encode, Decode, DecodeAll };

fn main() {
    let array = [0u8, 1u8, 2u8, 3u8];
    let value: u32 = 50462976;

    println!("{:02x?}", array.encode());
    println!("{:02x?}", value.encode());
    println!("{:?}", u32::decode(&mut &array.encode()[..]));
    println!("{:?}", u16::decode(&mut &array.encode()[..]));
    println!("{:?}", u16::decode_all(&mut &array.encode()[..]));
    println!("{:?}", u64::decode(&mut &array.encode()[..]));
}
[00, 01, 02, 03]
[00, 01, 02, 03]
Ok(50462976)
Ok(256)
Err(Error { cause: None, desc: "Input buffer has still data left after decoding!" })
Err(Error { cause: None, desc: "Not enough data to fill buffer" })
```
## 1.1 Depth Limit
Greater complexity in the decode type leads to increased computational resources used for value decoding. Generally you always want to `decode_with_depth_limit`. Substrate uses a limit of `256`.

```rust
use parity_scale_codec_derive::{Encode, Decode};
use parity_scale_codec::{Encode, Decode, DecodeLimit};

#[derive(Encode, Decode, Debug)]
enum Example {
    First,
    Second(Box<Self>),
}

fn main() {
    let bytes = vec![1, 1, 1, 1, 1, 0];
    println!("{:?}", Example::decode(&mut &bytes[..]));
    println!("{:?}", Example::decode_with_depth_limit(10, &mut &bytes[..]));
    println!("{:?}", Example::decode_with_depth_limit(3, &mut &bytes[..]));
}
Ok(Second(Second(Second(Second(Second(First))))))
Ok(Second(Second(Second(Second(Second(First))))))
Err(Error { cause: Some(Error { cause: Some(Error { cause: Some(Error { cause: Some(Error { cause: None, desc: "Maximum recursion depth reached when decoding" }), desc: "Could not decode `Example::Second.0`" }), desc: "Could not decode `Example::Second.0`" }), desc: "Could not decode `Example::Second.0`" }), desc: "Could not decode `Example::Second.0`" })
```

## 1.2 When One-to-One Decoding Fails: `BTreeSet`

SCALE is intended to be a one-to-one encoding, meaning the decoding process should return the exact data that was initially encoded. However, a notable exception occurs when using a `BTreeSet`.
 
In Rust, a `BTreeSet` is a set data structure implemented using a B-tree, which keeps its elements sorted. This ordering is part of the internal functionality of the `BTreeSet` and doesn't usually concern users directly. However, this characteristic comes into play when encoding and then decoding data with SCALE. Consider the following example:
```rust
use parity_scale_codec::{ Encode, Decode, alloc::collections::BTreeSet };

fn main() {
    let vector = vec![4u8, 3u8, 2u8, 1u8, 0u8];
    let vector_encoded = vector.encode();
    let btree = BTreeSet::<u8>::decode(&mut &vector_encoded[..]).unwrap();
    let btree_encoded = btree.encode();

    println!("{:02x?}", vector_encoded);
    println!("{:02x?}", btree_encoded);
}
[14, 04, 03, 02, 01, 00]
[14, 00, 01, 02, 03, 04]
```
In this code, a vector of numbers is encoded, and then decoded into a `BTreeSet`. When the resulting `BTreeSet` is encoded again, the resulting data differs from the original encoded vector. This happens because the `BTreeSet` automatically sorts its elements upon decoding, resulting in a different ordering.

It is essential to be aware of this behavior when using `BTreeSets` and similar datatypes in your Substrate code. Remember, SCALE encoding/decoding aims to be one-to-one, but the automated sorting feature of the `BTreeSet` breaks this expectation. This is not a failure of SCALE but a feature of the `BTreeSet` type itself.

