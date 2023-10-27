---
title: "Encode"
weight: 2
# bookFlatSection: false
# bookToc: true
# bookHidden: false
# bookCollapseSection: false
# bookComments: false
# bookSearchExclude: false
math: true
---

# 1 Little-endian Encoding

SCALE encoded types are stored in little-endian (LE) mode. Little-endian systems store the least significant byte at the smallest memory address. In contrast, big-endian (BE) systems store the most significant byte at the smallest memory address. For example, consider the `u32` integer given by the hexadecimal representation $\text{0x0a0b0c0d}$. In LE systems the least significant byte, that is $\text{0x0d}$, would be stored at the smallest memory address. The next diagram illustrates how the integer would be stored in memory for the two different modes.

| Memory Address | $a$ | $a+1$ | $a+2$ | $a+3$ | ... |
|----------------|---|-----|-----|-----|-----|
|LE mode | $\text{0x0d}$ | $\text{0x0c}$ | $\text{0x0b}$ | $\text{0x0a}$ |
|BE mode | $\text{0x0a}$ | $\text{0x0b}$ | $\text{0x0c}$ | $\text{0x0d}$ |

It's important to understand that endianness isn't a property of numbers themselves, and therefore, it isn't reflected in their binary or hexadecimal representations. However, Rust provides methods to handle endianness explicitly. For instance, the `to_le_bytes()` method can be used to obtain a little-endian byte vector representation of an integer. See the example below:

```rust
fn main() {
    println!("{:b}", 42u16);
    println!("{:02x?}", 42u16.to_le_bytes());
    println!("{:b}", 16777215u32);
    println!("{:02x?}", 16777215u32.to_le_bytes());
}

101010
[2a, 00]
111111111111111111111111
[ff, ff, ff, 00]
```
In analogy to how the data would be stored in memory, the least significant byte is stored at the smallest vector index. Of course, this is only useful once the type is bigger than one byte.

# 2 SCALE Encoding Basics
## 2.1 Introduction
This section offers practical examples to help you understand how to encode your types using SCALE. SCALE encoding is facilitated by the `Encode` trait offered by the `parity-scale-codec` crate. For an overview of how common types are encoded by SCALE, please refer to the [specification]({{< ref "/specification" >}}).

To obtain the SCALE-encoded bytes as a `Vec<u8>`, use the `encode()` method on types that implement the `Encode` trait. For composite types such as structs and enums, you must first derive the `Encode` trait using the `parity-scale-codec-derive` crate before attempting to encode.

```rust
use parity_scale_codec::Encode;
use parity_scale_codec_derive::Encode;

#[derive(Encode)]
struct Example {
    number: u8,
    is_cool: bool,
    optional: Option<u32>,
}

fn main() {
    let my_struct = Example {
        number: 0,
        is_cool: true,
        optional: Some(69),
    };
    println!("{:02x?}", [0u8, 1u8, 2u8, 3u8, 4u8].encode());
    println!("{:02x?}", (0u8, true, Some(69u32)).encode());
    println!("{:02x?}", my_struct.encode());
}
[00, 01, 02, 03, 04]
[00, 01, 01, 45, 00, 00, 00]
[00, 01, 01, 45, 00, 00, 00]
```

In this example, the output demonstrates the encoded form of a byte array, a tuple, and a struct, all in hexadecimal format.

## 2.2 Compact Integer Encoding

Unsigned integers between $0$ and $2^{536} - 1$ can on average be more efficiently encoded using SCALE's compact encoding. While the ordinary fixed-width integer encoding depends on the size of the given integer's type (e.g. `u8`, `u16`, `u32`, ...), the compact encoding only looks at the number itself and disregards the type information. For example, the compact encodings of the integers `60u8`, `60u16` and `60u32` are all the same: $\text{Enc}\_{\text{SC}}^{\text{Comp}}(60) = \text{[0xf0]}$.

Compact encoding can be utilized by enclosing the number within the `Compact` struct, as illustrated in the example below.

```rust
use parity_scale_codec::{Compact, Encode};

fn main() {
    println!("{:02x?}", 0u8.encode());
    println!("{:02x?}", 0u16.encode());
    println!("{:02x?}", 0u32.encode());
    println!("{:02x?}", Compact(60u8).encode());
    println!("{:02x?}", Compact(60u16).encode());
    println!("{:02x?}", Compact(60u32).encode());
}
[00]
[00, 00]
[00, 00, 00, 00]
[f0]
[f0]
[f0]
```

There are four different modes in compact encoding. Which mode is used is automatically determined dependent on the size of the given integer. A quick overview of the cases is covered in the [specification](/docs/specification). Each mode is specified using a bit flag appended as the least significant two bits of the compact encoding.

| Mode | Single-byte | Two-byte | Four-byte | Big-integer |
| -- | -- | -- | -- | -- | 
| Bit flag | 00 | 01 | 10 | 11 |

The following section provides more in-depth examples of how the different modes operate. It's not strictly necessary to understand how the values are encoded to use compact encoding. Since the appropriate mode is deduced automatically during encoding, there's no difference in usage from the perspective of a Rust user.

In what follows $\text{0x}$ indicates the hexadecimal representation of a number and $\text{0b}$ indicates its binary representation.

### 2.4.1 Single-byte mode
In this mode a given non-negative integer $n$ is encoded as a single byte. This is possible for $0 \leq n \leq 63$. Here the six most significant bits are the encoding of the value. As an example, consider the case $n = 42 = 2^5 + 2^3 + 2^1$. The compact encoding of $n$ is obtained by appending ${\color{red}00}$ as its least significant bits:

$$ 42 = 0b101010 \Longrightarrow 0b101010{\color{red}00} = 168$$

Therefore, the compact encoding of $n$ is given by the byte array $\text{Enc}\_{\text{SC}}^{\text{Comp}}(n) = \text{[0xa8]}$. Since there's only one byte in this mode the LE aspect cannot be seen.

```rust
use parity_scale_codec::{Compact, Encode};

fn main() {
    println!("{:02x?}", 42u8.encode());
    println!("{:02x?}", 42u32.encode());
    println!("{:02x?}", Compact(42u8).encode());
    println!("{:02x?}", Compact(42u32).encode());
}
[2a]
[2a, 00, 00, 00]
[a8]
[a8]
```

### 2.4.2 Two-byte mode
In this mode two bytes are used for the encoding. The six most significant bits of the first byte and the following byte are the LE encoding of the value. It applies for $2^6 \leq n \leq 2^{14} -1$. Consider the case $n = 69 = 2^6 + 2^2 + 2^0$. The compact encoding of $n$ is obtained by appending ${\color{red}01}$ as its least significant bits:
$$ 69 = 0b1000101 \Longrightarrow 0b1000101{\color{red}01} = 277.$$
Since the resulting integer exceeds one byte, the number is split up starting with the least-significant byte. The compact encoding $\text{Enc}\_{\text{SC}}^{\text{Comp}}(n)$ is given by the byte array:
$$ 0b00000001\\;00010101 = \text{[0x15, 0x01]}.$$

```rust
use parity_scale_codec::{Compact, Encode};

fn main() {
    println!("{:02x?}", 69u8.encode());
    println!("{:02x?}", 69u32.encode());
    println!("{:02x?}", Compact(69u8).encode());
    println!("{:02x?}", Compact(69u32).encode());
}
[45]
[45, 00, 00, 00]
[15, 01]
[15, 01]
```

### 2.4.3 Four-byte mode
This mode uses four bytes to encode the value, which happens when $2^{14} \leq n \leq 2^{30} - 1$. Consider the case $n = 2^{16} - 1 = 65535$. This is the maximum value for the type `u16`. Its compact encoding is obtained by appending ${\color{red}10}$ as its least significant bits:
$$ 65535 = 0b11111111\\;11111111 \Longrightarrow 0b11111111\\;11111111{\color{red}10} = 262142.$$
Analogously to the previous example, the resulting integer exceeds two bytes and needs to be split up using little-endian mode. Additionally, we pad with leading zeros. The compact encoding $\text{Enc}\_{\text{SC}}^{\text{Comp}}(n)$ is given by the byte array:
$$ 0b00000000\\;00000011\\;11111111\\;11111110 = \text{[0xfe, 0xff, 0x03, 0x00]}.$$
```rust
use parity_scale_codec::{Compact, Encode};

fn main() {
    println!("{:02x?}", 65535u16.encode());
    println!("{:02x?}", 65535u32.encode());
    println!("{:02x?}", Compact(65535u16).encode());
    println!("{:02x?}", Compact(65535u32).encode());
}
[ff, ff]
[ff, ff, 00, 00]
[fe, ff, 03, 00]
[fe, ff, 03, 00]
```
### 2.4.4 Big-integer mode

This mode is intended for non-negative integers between $2^{30}$ and $2^{536} - 1$. It differs from the other three modes in that it is a variable length encoding. As a first example, consider the case $n = 2^{30} = 1073741824$. This number's LE encoding is given by:
$$0b 01000000\\;00000000\\;00000000\\;00000000 = \text{[0x00, 0x00, 0x00, 0x40]}.$$

Now, in big-integer mode, the six most significant bits of the first byte are used to store the number of bytes $m$ used in the actual encoding of the number *minus four*. That is $m - 4$. Since the LE encoding of $n$ is exactly of length $m = 4$, the upper six bits of the first byte must all be equal to zero. In accordance with the other cases, the mode is indicated using the two least significant bits of the first byte. For big-integer mode we append ${\color{red}11}$ to obtain as the first byte:
$$ 0 = m - 4 = 0b000000 \Longrightarrow 0b000000{\color{red}11} = 3.$$
In total, the compact encoding $\text{Enc}\_{\text{SC}}^{\text{Comp}}(n)$ is given by the byte array:
$$\text{[0x03, 0x00, 0x00, 0x00, 0x40]}.$$

Let's look at another example. The LE encoding of the number $n = 2^{32} = 4294967296$ is given by $\text{[0x00, 0x00, 0x00, 0x00, 0x01]}.$ This time we need five bytes to store it, i.e. $m = 5$. Again, we use six bits to encode this length *minus four* and after that append ${\color{red}11}$:
$$ 1 = m - 4 = 0b000001 \Longrightarrow 0b000001{\color{red}11} = 7. $$
Altogether, the compact encoding $\text{Enc}\_{\text{SC}}^{\text{Comp}}(n)$ is given by the byte array:
$$\text{[0x07, 0x00, 0x00, 0x00, 0x00, 0x01]}.$$

{{< hint info >}}
**Note**: The rationale behind storing $m-4$, rather than $m$ directly, lies in maximizing the efficiency of the available six bits, given that these bits set the limit for the size of integers we can compact encode. The smallest integer in big-integer mode, $2^{30}$, has a LE encoding that consists of $4$ bytes. Encoding this as $0b000010{\color{red}11}$ would inefficiently utilize the available space. By choosing to encode $m - 4$ instead, the first six bits can accommodate a length of $63 + 4$. This approach allows for the encoding of integers up to $2^{(63+4)8} - 1 = 2^{536} - 1$.
{{< /hint >}}


```rust
use parity_scale_codec::{Compact, Encode};

fn main() {
    println!("{:02x?}", 1073741824u32.encode());
    println!("{:02x?}", 4294967296u64.encode());
    println!("{:02x?}", Compact(1073741824u32).encode());
    println!("{:02x?}", Compact(4294967296u64).encode());
}
[00, 00, 00, 40]
[00, 00, 00, 00, 01, 00, 00, 00]
[03, 00, 00, 00, 40]
[07, 00, 00, 00, 00, 01]
```

## 2.3 Embedding Compact Encodings
We can also embed compact integer encodings within other types to make them more efficient.

### 2.3.1 Structs

By using the `codec(compact)` attribute of the `derive` macro we can specify that selected fields within a `struct` type will be compactly encoded. For example, in the following snippet we marked the `compact_number` field of the `Example` struct to be compactly encoded.

```rust
use parity_scale_codec_derive::Encode;
use parity_scale_codec::Encode;

#[derive(Encode)]
struct Example {
    number: u64,
    #[codec(compact)]
    compact_number: u64,
}

fn main() {
    let my_struct = Example { number: 42, compact_number: 1337 };
    println!("{:02x?}", my_struct.encode());
}
[2a, 00, 00, 00, 00, 00, 00, 00, e5, 14]
```

### 2.3.2 Enums
We can proceed similarly with `enums`. In this snippet only the second `u64` of the `One` variant will be compactly encoded.

```rust
use parity_scale_codec_derive::Encode;
use parity_scale_codec::Encode;

#[derive(Encode)]
enum Choices {
    One(u64, #[codec(compact)] u64),
}

fn main() {
    let my_choice = Choices::One(42, 1337);
    println!("{:02x?}", my_choice.encode());
}
[00, 2a, 00, 00, 00, 00, 00, 00, 00, e5, 14]
```