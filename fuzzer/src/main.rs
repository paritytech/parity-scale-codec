use std::collections::{BTreeMap, BTreeSet, VecDeque, LinkedList, BinaryHeap};
use std::time::Duration;

use bitvec::{vec::BitVec, cursor::BigEndian};
use honggfuzz::fuzz;
use parity_scale_codec::{Encode, Decode, Compact};
use honggfuzz::arbitrary::Arbitrary;

#[derive(Encode, Decode, Clone, PartialEq, Debug, Arbitrary)]
pub struct MockStruct{
	vec_u: Vec<u8>
}

#[derive(Encode, Decode, Debug, Clone, Arbitrary)]
struct BinaryHeapWrapper(BinaryHeap<u32>);

impl PartialEq for BinaryHeapWrapper {
	fn eq(&self, other: &BinaryHeapWrapper) -> bool {
		let a = self.0.iter().cloned().collect::<Vec<u32>>().sort();
		let b = other.0.iter().cloned().collect::<Vec<u32>>().sort();
		a == b
	}
}

// #[derive(Encode, Decode, PartialEq, Debug, Clone, Arbitrary, Cursor, BitStore)]
// pub struct BigEndianWrapper(BigEndian);

// #[derive(Encode, Decode, PartialEq, Debug, Clone, Arbitrary)]
// pub struct BitVecWrapper<T: BitStore>(BitVec<BigEndianWrapper, T>);

#[derive(Encode, Decode, Clone, PartialEq, Debug, Arbitrary)]
pub enum MockEnum {
	Empty,
	Unit(u32),
	UnitVec(Vec<u8>),
	Complex {
		data: Vec<u32>,
		map: BTreeMap<[u8; 32], Vec<u8>>,
		string: String,
	},
	Mock(MockStruct),
	NestedVec(Vec<Vec<Vec<Vec<Vec<Vec<Vec<Vec<Option<u8>>>>>>>>>),
}

macro_rules! fuzz_decoder {
	(
		$data:ident;
		$first:ty,
		$( $rest:ty, )*
	) => {
		fuzz_decoder! {
			@INTERNAL
			$data;
			1u8;
			{ $first; 0u8 }
			$( $rest, )*
		}
	};
	(@INTERNAL
		$data:ident;
		$counter:expr;
		{ $( $parsed:ty; $index:expr ),* }
		$current:ty,
		$( $rest:ty, )*
	) => {
		fuzz_decoder! {
			@INTERNAL
			$data;
			$counter + 1u8;
			{ $current; $counter $(, $parsed; $index )* }
			$( $rest, )*
		}
	};
	(@INTERNAL
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
			if let Ok(obj) = maybe_obj {
				let mut d2: &[u8] = &obj.encode();
				let raw2 = d2.clone();
				let exp_obj = <$parsed>::decode(&mut d2);
				match exp_obj {
					Ok(obj2) => {
						if obj == obj2 {
							let raw1_trunc_to_obj_size = &raw1[..raw1.len() - d.len()];
							if raw1_trunc_to_obj_size != raw2 {
								println!("Type: {}", std::any::type_name::<$parsed>());
								println!("raw1 = {:?}", raw1);
								println!("d (leftover/undecoded data) = {:?}", d);
								println!("- Decoded data:");
								println!("raw1_trunc = {:?}", raw1_trunc_to_obj_size);
								println!("raw2 = {:?}", raw2);
								println!("- Encoded objects:");
								println!("obj1 = '{:?}'", obj);
								println!("obj2 = '{:?}'", obj2);
								panic!("raw1 != raw2");
							}
						return
						}
					panic!("obj != obj2; obj={:?}, obj2={:?}", obj, obj2);
					},
					Err(e) => {
						panic!("Shouldn’t happen: can't .decode() after .decode().encode(): {}", e);
					}
				}
			}
			return
		}
	)*

		unreachable!()
	};
}

fn fuzz_decode(data: &[u8]) {
	fuzz_decoder! {
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
		BTreeMap<String, Vec<u8>>,
		BTreeMap<u8, u8>,
		BTreeSet<u32>,
		VecDeque<u8>,
		BinaryHeapWrapper,
		MockStruct,
		MockEnum,
		BitVec<BigEndian, u8>,
		BitVec<BigEndian, u32>,
		Duration,
	};
}

macro_rules! fuzz_encoder {
	(
		$first:ty,
		$( $rest:ty, )*
	) => {
		fuzz_encoder! {
			@INTERNAL
			1u8;
			{ $first; 0u8 }
			$( $rest, )*
		}
	};
	(@INTERNAL
		$counter:expr;
		{ $( $parsed:ty; $index:expr ),* }
		$current:ty,
		$( $rest:ty, )*
	) => {
		fuzz_encoder! {
			@INTERNAL
			$counter + 1u8;
			{ $current; $counter $(, $parsed; $index )* }
			$( $rest, )*
		}
	};
	(@INTERNAL
		$counter:expr;
		{ $( $parsed:ty; $index:expr ),* }
	) => {
	$(
		fuzz!(|data: $parsed| { fuzz_encode(data) });
	)*
	};
}

fn fuzz_encode<T: Encode + Decode + Clone + PartialEq + std::fmt::Debug> (data: T) {
	let original = data.clone();
	let mut obj: &[u8] = &data.encode();
	let decoded = <T>::decode(&mut obj);
	if let Ok(object) = decoded {
		if object != original {
			println!("original object: {:?}", original);
			println!("decoded object: {:?}", object);
			panic!("Original object differs from decoded object")
		}
	} else {
		// safe because we checked that object is not Ok
		let e = decoded.unwrap_err();
		println!("original object: {:?}", original);
		println!("decoding error: {:?}", e);
		panic!("Failed to decode the encoded object");
	}
}

macro_rules! tmp {
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
		// BitVec<BigEndian, u8>,
		// BitVec<BigEndian, u32>,
		Duration,
		}
	};
}

fn main() {
	loop {
		fuzz!(|data: &[u8]| { fuzz_decode(data); });
		tmp!();
	}
}
