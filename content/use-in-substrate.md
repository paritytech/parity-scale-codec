---
title: "Use in Substrate"
weight: 4
# bookFlatSection: false
# bookToc: true
# bookHidden: false
# bookCollapseSection: false
# bookComments: false
# bookSearchExclude: false
math: true
---

# 1. Using SCALE in Substrate Development

## 1.1 General Workflow

Pallets interact with the SCALE codec when their data structures need to be serialized for storage or network transmission, or deserialized for processing. The usage of SCALE in pallet and runtime development is straightforward and usually handled by simply deriving `Encode` and `Decode` for your data types.

## 1.2 Case Study: Balances Pallet

We illustrate this approach using an example taken from the [balances pallet](https://docs.rs/pallet-balances/22.0.0/pallet_balances/). 

First, the `AccountData` struct is defined in `types.rs`, with `Encode`, `Decode` and some other traits derived. This allows it to be automatically encoded and decoded when stored in Substrate's storage or when being part of the event parameters.

```rust
/// All balance information for an account.
#[derive(Encode, Decode, Clone, PartialEq, Eq, Default, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct AccountData<Balance> {
	pub free: Balance,
	pub reserved: Balance,
	pub frozen: Balance,
	pub flags: ExtraFlags,
}
```

Next, the `balances` pallet uses the `AccountData` struct to represent all balance information for a given account. This data is stored in the `Account` storage map, where each `AccountId` is mapped to its corresponding `AccountData`.

```rust
#[pallet::storage]
pub type Account<T: Config<I>, I: 'static = ()> =
    StorageMap<_, Blake2_128Concat, T::AccountId, AccountData<T::Balance>, ValueQuery>;
```
The `Account` storage map is part of the pallet's storage and defined within the `#[pallet::storage]` macro of the `lib.rs` file. With the `Encode` and `Decode` traits derived for `AccountData`, any data written to or read from this storage map will be automatically encoded or decoded.

## 1.3 Automatic Decoding in Action

When the balances pallet needs to read an account's balance from storage, the decoding happens automatically. Here is the `balance` function from the Balances pallet:

```rust
fn balance(who: &T::AccountId) -> Self::Balance {
	Self::account(who).free
}
```
This function retrieves the `AccountData` of the given account from the storage, then returns the `free` balance field of the struct. The function chain involved in fetching the data from storage, decoding it, and accessing the data fields is abstracted away by the Substrate framework, demonstrating the utility of SCALE and Substrate's storage APIs. 

By following this pattern - defining your data types, deriving the appropriate traits, and using Substrate's storage APIs - you can seamlessly work with serialized data in your pallet development, keeping the complexity of serialization and deserialization hidden away.

# 2. Common Patterns

The following section introduces some important patterns used in Substrate. For a comprehensive list of traits employed in SCALE please refer to the [SCALE rust docs](https://docs.rs/parity-scale-codec/latest/parity_scale_codec/).

## 2.1 The `MaxEncodedLen` Trait
The `MaxEncodedLen` trait is an important part of the SCALE encoding system utilized in Substrate. It provides a method for defining the maximum length, in bytes, that a type will take up when it is SCALE-encoded. This is particularly useful for putting an upper limit on the size of encoded data, enabling checks against this maximum length to reject overly large data.

```rust
pub trait MaxEncodedLen: Encode {
	/// Upper bound, in bytes, of the maximum encoded size of this item.
	fn max_encoded_len() -> usize;
}
```
A concrete example of its usage can be seen in Substrate's [democracy pallet](https://paritytech.github.io/substrate/master/pallet_democracy/index.html), specifically in how it is implemented for the `Vote` struct:
```rust
/// A number of lock periods, plus a vote, one way or the other.
#[derive(Copy, Clone, Eq, PartialEq, Default, RuntimeDebug)]
pub struct Vote {
	pub aye: bool,
	pub conviction: Conviction,
}
```
The `Vote` struct contains two fields: `aye` (a boolean indicating a positive or negative vote) and `conviction` (an enum indicating the conviction level of the vote with $7$ variants). Despite the presence of multiple enum variants and a boolean, the Democracy pallet implements the `MaxEncodedLen` trait for Vote to fit within $1$ byte:
```rust
impl MaxEncodedLen for Vote {
	fn max_encoded_len() -> usize {
		1
	}
}
```
The encoding scheme for the `Vote` struct involves a clever utilization of the single byte's capacity. Here's how the `Vote` struct's `Encode` trait is implemented:
```rust
impl Encode for Vote {
	fn encode_to<T: Output + ?Sized>(&self, output: &mut T) {
		output.push_byte(u8::from(self.conviction) | if self.aye { 0b1000_0000 } else { 0 });
	}
}
```
In this custom `Encode` implementation, the `Conviction` enum, which is represented as a `u8`, is encoded into the least significant $3$ bits of the byte. The `aye` bool, denoting whether the vote is in favor or against, is encoded into the most significant bit of the byte. If the vote is in favor (`aye` is true), the bit is set to $1$ ($0b10000000$ in binary), and if the vote is against (`aye` is false), the bit is set to $0$.

This way, both `aye` and `conviction` are packed together into a single byte, ensuring that the data structure remains as compact as possible. This example demonstrates how the `MaxEncodedLen` trait can be effectively used to control the size of encoded data in Substrate pallet development.
