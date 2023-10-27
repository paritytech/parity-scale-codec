---
title: "Index"
weight: 1
# bookFlatSection: false
# bookToc: true
# bookHidden: false
# bookCollapseSection: false
# bookComments: false
# bookSearchExclude: false
---

![logo](logo.png)
{{< hint info >}}
**SCALE** (**S**imple **C**oncatenated **A**ggregate **L**ittle-**E**ndian) is the data format for types used in the Parity Substrate framework. It is a light-weight format which allows encoding (and decoding) which makes it highly suitable for resource-constrained execution environments like blockchain runtimes and low-power, low-memory devices.
{{< /hint >}}

Welcome to the technical documentation of the [Rust implementation](https://github.com/paritytech/parity-scale-codec) of Parity's SCALE codec. This page is intended to serve as an introduction to SCALE for those new to Substrate development. For more detailed, low-level information about the `parity-scale-codec` Rust crate, please visit the corresponding [docs.rs page](https://docs.rs/parity-scale-codec/latest/parity_scale_codec/).

This page is divided into the following sections:
- **Encode**: This section provides a practical introduction to how SCALE is used to encode types in Rust, complete with examples. It is recommended to read this section before proceeding to the Decode section.
- **Decode**: This section explains how to decode SCALE-encoded data and addresses common misconceptions and challenges related to SCALE's non-descriptive nature.
- **Use in Substrate**: This section outlines how SCALE is utilized in Substrate development and showcases common patterns.
- **Specification**: This section offers a brief overview of the SCALE encoding process.
- **SCALE crates**: This page provides a high-level overview of the various available SCALE Rust crates and their uses.

SCALE is non-descriptive. This means that the encoding context, which includes knowledge of the types and data structures, must be known separately at both the encoding and decoding ends. The encoded data does not include this contextual information. Consider the following comparison between SCALE and JSON to understand what this means in practice. 
{{< tabs "SCALEvsJSON" >}}
{{< tab "SCALE" >}}
```rust
use parity_scale_codec::{ Encode };

#[derive(Encode)]
struct Example {
    number: u8,
    is_cool: bool,
    optional: Option<u32>,
}

fn main() {
    let my_struct = Example {
        number: 42,
        is_cool: true,
        optional: Some(69),
    };
    println!("{:?}", my_struct.encode());
    println!("{:?}", my_struct.encode().len());
}
[42, 1, 1, 69, 0, 0, 0]
7
```
{{< /tab >}}
{{< tab "JSON" >}}
```rust
use serde::{ Serialize };

#[derive(Serialize)]
struct Example {
    number: u8,
    is_cool: bool,
    optional: Option<u32>,
}

fn main() {
    let my_struct = Example {
        number: 42,
        is_cool: true,
        optional: Some(69),
    };
    println!("{:?}", serde_json::to_string(&my_struct).unwrap());
    println!("{:?}", serde_json::to_string(&my_struct).unwrap().len());
}
"{\"number\":42,\"is_cool\":true,\"optional\":69}"
42
```
{{< /tab >}}
{{< /tabs >}}