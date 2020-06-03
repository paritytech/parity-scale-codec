use std::collections::{BTreeMap, BTreeSet, VecDeque, LinkedList, BinaryHeap};
use std::time::Duration;

use bitvec::{vec::BitVec, order::Msb0, order::BitOrder, store::BitStore};
use honggfuzz::fuzz;
use parity_scale_codec::{Encode, Decode, Compact};
use honggfuzz::arbitrary::{Arbitrary, Unstructured, Result as ArbResult};

#[derive(Encode, Decode, Clone, PartialEq, Debug, Arbitrary)]
pub struct MockStruct{
	vec_u: Vec<u8>
}

/// Used for implementing the Arbitrary trait for a BitVec.
#[derive(Encode, Decode, Clone, PartialEq, Debug)]
pub struct BitVecWrapper<O: BitOrder, T: BitStore>(BitVec<O, T>);

impl<O: 'static + BitOrder, T: 'static + BitStore + Arbitrary> Arbitrary for BitVecWrapper<O, T> {
	fn arbitrary(u: &mut Unstructured<'_>) -> ArbResult<Self> {
		let v = Vec::<T>::arbitrary(u)?;
		Ok(BitVecWrapper(BitVec::<O, T>::from_vec(v)))
	}
}


/// Used for implementing the PartialEq trait for a BinaryHeap.
#[derive(Encode, Decode, Debug, Clone, Arbitrary)]
struct BinaryHeapWrapper(BinaryHeap<u32>);

impl PartialEq for BinaryHeapWrapper {
	fn eq(&self, other: &BinaryHeapWrapper) -> bool {
		self.0.clone().into_sorted_vec() == other.0.clone().into_sorted_vec()
	}
}

#[derive(Encode, Decode, Clone, PartialEq, Debug, Arbitrary)]
pub enum MockEnum {
	Empty,
	Unit(u32),
	UnitVec(Vec<u8>),
	Complex {
		data: Vec<u32>,
		bitvec: BitVecWrapper<Msb0, u8>,
		string: String,
	},
	Mock(MockStruct),
	NestedVec(Vec<Vec<Vec<Vec<Vec<Vec<Vec<Vec<Option<u8>>>>>>>>>),
}

/// `fuzz_flow` parameter can either be `round_trip` or `only_decode`.
/// `round_trip` will decode -> encode and compare the obtained encoded bytes with the original data.
/// `only_decode` will only decode, without trying to encode the decoded object.
/// `round_trip_sort` will decode -> encode and compare the obtained encoded SORTED bytes with the original SORTED data.
macro_rules! fuzz_decoder {
	(
		$fuzz_flow:ident;
		$data:ident;
		$first:ty,
		$( $rest:ty, )*
	) => {
		fuzz_decoder! {
			@INTERNAL
			$fuzz_flow;
			$data;
			1u8;
			{ $first; 0u8 }
			$( $rest, )*
		}
	};
	(@INTERNAL
		$fuzz_flow:ident;
		$data:ident;
		$counter:expr;
		{ $( $parsed:ty; $index:expr ),* }
		$current:ty,
		$( $rest:ty, )*
	) => {
		fuzz_decoder! {
			@INTERNAL
			$fuzz_flow;
			$data;
			$counter + 1u8;
			{ $current; $counter $(, $parsed; $index )* }
			$( $rest, )*
		}
	};
	// round_trip flow arm.
	(@INTERNAL
		round_trip;
		$data:ident;
		$counter:expr;
		{ $( $parsed:ty; $index:expr ),* }
	) => {
		let num = $counter;
	$(
		if $data[0] % num == $index {
			let mut d = &$data[1..];
			let raw1 = d.clone();
			let maybe_obj = <$parsed>::decode(&mut d);

			match maybe_obj {
				Ok(obj) => {
					let mut d2: &[u8] = &obj.encode();
					let raw2 = d2.clone();
					let exp_obj = <$parsed>::decode(&mut d2);
					match exp_obj {
						Ok(obj2) => {
							if obj == obj2 {
								let raw1_trunc_to_obj_size = &raw1[..raw1.len()-d.len()];
								if raw1_trunc_to_obj_size != raw2 {
									println!("raw1 = {:?}", raw1);
									println!("d (leftover/undecoded data) = {:?}", d);
									println!("- Decoded data:");
									println!("raw1_trunc = {:?}", raw1_trunc_to_obj_size);
									println!("raw2 = {:?}", raw2);
									println!("- Encoded objects:");
									println!("obj1 = '{:?}'", obj);
									println!("obj2 = '{:?}'", obj2);
									println!("Type: {}", std::any::type_name::<$parsed>());
									panic!("raw1 != raw2");
								}
								return
							} else {
								panic!("obj != obj2; obj={:?}, obj2={:?}", obj, obj2);
							}
						}
						Err(e) => panic!("Shouldn’t happen: can't .decode() after .decode().encode(): {}", e),
					}
				}
				Err(_) => return
			}
		}
	)*
	};
	// only_decode flow arm.
	(@INTERNAL
		only_decode;
		$data:ident;
		$counter:expr;
		{ $( $parsed:ty; $index:expr ),* }
	) => {
		let num = $counter;
		$(
			if $data[0] % num == $index {
				// Check that decode doesn't panic
				let _ = <$parsed>::decode(&mut &$data[1..]);
				return
			}
		)*
	};
	// round_trip_sorted flow arm.
	(@INTERNAL
		round_trip_sorted;
		$data:ident;
		$counter:expr;
		{ $( $parsed:ty; $index:expr ),* }
	) => {
		let num = $counter;
	$(
		if $data[0] % num == $index {
			let mut d = &$data[1..];
			let raw1 = &d.clone();

			let maybe_obj = <$parsed>::decode(&mut d);
			match maybe_obj {
				Ok(obj) => {
					let d2 = obj.encode();
					let mut raw2 = d2.clone();
					// We are sorting here because we're in the "sorted" flow. Useful for container types
					// which can have multiple valid encoded versions.
					raw2.sort();
					let exp_obj = <$parsed>::decode(&mut &d2[..]);
					match exp_obj {
						Ok(obj2) => {
							if obj == obj2 {
								let mut raw1_trunc_to_obj_size = Vec::from(&raw1[..raw1.len() - d.len()]);
								// Sorting here is necessary: see above comment.
								raw1_trunc_to_obj_size.sort();
								if raw1_trunc_to_obj_size != raw2 {
									println!("raw1 = {:?}", raw1);
									println!("d (leftover/undecoded data) = {:?}", d);
									println!("- Decoded data:");
									println!("raw1_trunc = {:?}", raw1_trunc_to_obj_size);
									println!("raw2 = {:?}", raw2);
									println!("- Encoded objects:");
									println!("obj1 = '{:?}'", obj);
									println!("obj2 = '{:?}'", obj2);
									println!("Type: {}", std::any::type_name::<$parsed>());
									panic!("raw1 != raw2");
								}
								return
							}
							panic!("obj != obj2; obj={:?}, obj2={:?}", obj, obj2);
						},
						Err(e) => panic!("Shouldn’t happen: can't .decode() after .decode().encode(): {}", e),
					}
				}
				Err(_) => return,
			}
		}
	)*
	};
}

fn fuzz_decode(data: &[u8]) {
	// Types for which we wish to apply the "round_trip" method.
	fuzz_decoder! {
		round_trip;
		data;
		u8,
		u16,
		u32,
		u64,
		u128,
		Compact<u8>,
		Compact<u16>,
		Compact<u32>,
		Compact<u64>,
		Compact<u128>,
		String,
		Vec<u8>,
		Vec<Vec<u8>>,
		Option<Vec<u8>>,
		Vec<u32>,
		LinkedList<u8>,
		VecDeque<u8>,
		MockStruct,
		MockEnum,
		BitVec<Msb0, u8>,
		BitVec<Msb0, u32>,
		Duration,
	};
	// Types for which we wish to apply the "sorted" method.
	fuzz_decoder! {
		round_trip_sorted;
		data;
		BinaryHeapWrapper,
	};
	// Types for which we only wish to decode.
	fuzz_decoder! {
		only_decode;
		data;
		BTreeMap<String, Vec<u8>>,
		BTreeMap<u8, u8>,
		BTreeSet<u32>,
	};
}

macro_rules! fuzz_encoder {
	() => {};
	($( $type:ty, )*) => {
		$(fuzz!(|data: $type| { fuzz_encode(data) });)*
	};
}

fn fuzz_encode<T: Encode + Decode + Clone + PartialEq + std::fmt::Debug> (data: T) {
	let original = data.clone();
	let mut obj: &[u8] = &data.encode();
	let decoded = <T>::decode(&mut obj);
	match decoded {
		Ok(object) => {
			if object != original {
				println!("original object: {:?}", original);
				println!("decoded object: {:?}", object);
				panic!("Original object differs from decoded object")
			}
		}
		Err(e) => {
			println!("original object: {:?}", original);
			println!("decoding error: {:?}", e);
			panic!("Failed to decode the encoded object");
		}
	}
}

macro_rules! fuzz_encoding {
	() => {
		fuzz_encoder! {
			u8,
			u16,
			u32,
			u64,
			u128,
			Compact<u8>,
			Compact<u16>,
			Compact<u32>,
			Compact<u64>,
			Compact<u128>,
			String,
			Vec<u8>,
			Vec<Vec<u8>>,
			Option<Vec<u8>>,
			Vec<u32>,
			LinkedList<u8>,
			BTreeMap<String, Vec<u8>>,
			BTreeMap<u8, u8>,
			BTreeSet<u32>,
			VecDeque<u8>,
			BinaryHeapWrapper,
			MockStruct,
			MockEnum,
			BitVecWrapper<Msb0, u8>,
			BitVecWrapper<Msb0, u32>,
			Duration,
		}
	};
}

fn main() {
	loop {
		fuzz!(|data: &[u8]| { fuzz_decode(data); });
		fuzz_encoding!();
	}
}
