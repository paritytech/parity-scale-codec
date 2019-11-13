use std::collections::{BTreeMap, BTreeSet, VecDeque, LinkedList, BinaryHeap};

use bitvec::{vec::BitVec, cursor::BigEndian};
use honggfuzz::fuzz;
use parity_scale_codec::{Encode, Decode, Compact};

#[derive(Encode, Decode)]
pub struct MockStruct{
	vec_u: Vec<u8>
}

#[derive(Encode, Decode)]
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

macro_rules! fuzz_types {
	(
		$data:ident;
		$first:ty,
		$( $rest:ty, )*
	) => {
		fuzz_types! {
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
		fuzz_types! {
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
				// Check that decode doesn't panic.
				let _ = <$parsed>::decode(&mut &$data[1..]);
				return
			}
		)*

		unreachable!()
	};
}

fn fuzz_one_input(data: &[u8]){
	fuzz_types! {
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
		BinaryHeap<u32>,
		MockStruct,
		MockEnum,
		BitVec<BigEndian, u8>,
		BitVec<BigEndian, u32>,
	}
}

fn main() {
	loop {
		fuzz!(|data: &[u8]| { fuzz_one_input(data); });
	}
}


