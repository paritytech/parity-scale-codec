---
title: "SCALE crates"
weight: 6
# bookFlatSection: false
bookToc: true
# bookHidden: false
# bookCollapseSection: false
# bookComments: false
# bookSearchExclude: false
math: true
---

# 1. `scale-info`

{{<mermaid>}}
flowchart RL
    SI[[scale-info]]
    SIT1(Registry)
    SIT2(TypeInfo)
    SIT1-- struct -->SI
    SIT2-- trait -->SI
{{</mermaid>}}

The `scale-info` Rust crate provides the essential tooling to handle type metadata in a compact, efficient manner compatible with the SCALE encoding. Its primary features include the `Registry` struct and the `TypeInfo` trait. 

The `Registry` serves as a database that associates each type with its corresponding metadata, providing a convenient and efficient means to access necessary type data. 

The `TypeInfo` trait, on the other hand, enables users to generate type information for any Rust type that implements this trait, which in turn can be registered in the `Registry`.

These features of `scale-info` provide the underpinning for flexible encoding and decoding, by allowing types to describe themselves in a way that can be exploited by encoding and decoding tools such as the `scale-decode` crate.

{{< hint info >}}
Further reading: {{< fontawesome "rust" >}}[docs.rs](https://docs.rs/scale-info/latest/scale_info/) | {{< fontawesome "github" >}}[Github](https://github.com/paritytech/scale-info) | {{< fontawesome "box" >}}[Crates.io](https://crates.io/crates/scale-info)
{{< /hint >}}

# 2. `scale-decode`

{{<mermaid>}}
flowchart TD
    SD[[scale-decode]]
    SDT1(Visitor)
    SDT1-- trait -->SD
    SDT4("decode_with_visitor
    (bytes, typeid, registry, visitor)")-- function -->SD
    SDT4-->Decode("SCALE bytes decoded into custom data structure")
{{</mermaid>}}

The `scale-decode` crate facilitates the decoding of SCALE-encoded bytes into custom data structures by using type information from a `scale-info` registry. By implementing the `Visitor` trait and utilizing the `decode_with_visitor` function, developers can map decoded values to their chosen types with enhanced flexibility.

{{< hint info >}}
Further reading: {{< fontawesome "rust" >}}[docs.rs](https://docs.rs/scale-decode/latest/scale_decode/) | {{< fontawesome "github" >}}[Github](https://github.com/paritytech/scale-decode) | {{< fontawesome "box" >}}[Crates.io](https://crates.io/crates/scale-decode/)
{{< /hint >}}

# 3. `scale-value`

{{<mermaid>}}
flowchart TD
    SD[[scale-value]]
    SDT1(Value)
    SDT1-- struct -->SD
{{</mermaid>}}

This crate provides a `Value` type, which is a runtime representation that is compatible with type descriptions from `scale-info`. It somewhat analogous to a `serde_json::Value`, which is a runtime representation of JSON values, but with a focus on SCALE encoded values instead of JSON encoded values. Unlike JSON however, SCALE encoding is not self describing, and so we need additional type information to tell us how to encode and decode values. It is expected that this crate will commonly be used in conjunction with the `scale-info` and `frame-metadata` crates.

{{< hint info >}}
Further reading: {{< fontawesome "rust" >}}[docs.rs](https://docs.rs/scale-value/latest/scale_value/) | {{< fontawesome "github" >}}[Github](https://github.com/paritytech/scale-value) | {{< fontawesome "box" >}}[Crates.io](https://crates.io/crates/scale-value)
{{< /hint >}}

# 4. `frame-metadata`

While not directly a part of SCALE, the `frame-metadata` crate utilizes a `Registry` from the `scale-info` crate. The `frame-metadata` crate provides a struct that encapsulates metadata about a Substrate runtime. A notable aspect of this struct is a type registry, which is a collection of all types utilized in the metadata of the runtime. In addition, the struct comprises comprehensive data on the runtime's pallets, extrinsics, Runtime API, outer enums, and even accommodates custom metadata. The collective use of these elements allows developers to effectively navigate and adapt to the intricacies of a specific Substrate-based blockchain runtime.

{{< hint info >}}
Further reading: {{< fontawesome "rust" >}}[docs.rs](https://docs.rs/frame-metadata/latest/frame_metadata/) | {{< fontawesome "github" >}}[Github](https://github.com/paritytech/frame-metadata) | {{< fontawesome "box" >}}[Crates.io](https://crates.io/crates/frame-metadata)
{{< /hint >}}