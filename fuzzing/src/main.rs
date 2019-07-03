use parity_scale_codec::{Encode, Decode};
use std::collections::{BTreeMap, BTreeSet, VecDeque, LinkedList, BinaryHeap};
#[cfg(not(fuzzing))]
use std::io::{self, Read};

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[derive(Debug)]
pub struct MockStruct{
   vec_u: Vec<u8>
}

fn fuzz_one_input(data: &[u8]){
    match data[0] % 9 {
        0 => { let _u_16 = u16::decode(&mut &data[1..]);},
        1 => { let _vec_u8= Vec::<u8>::decode(&mut &data[1..]);},
        2 => { let _vec_u32 = Vec::<u32>::decode(&mut &data[1..]);}
        3 => { let _linked_list = LinkedList::<u8>::decode(&mut &data[1..]);},
        4 => { let _btree = BTreeMap::<u8, u8>::decode(&mut &data[1..]);},
        5 => { let _btreeset = BTreeSet::<u8>::decode(&mut &data[1..]);},
        6 => { let _vecdeque = VecDeque::<u8>::decode(&mut &data[1..]);},
        7 => { let _binaryheap = BinaryHeap::<u8>::decode(&mut &data[1..]);},
        8 => { let _mock_struct = MockStruct::decode(&mut &data[1..]);}
        _ => unreachable!()
    }
}

#[macro_use] extern crate honggfuzz;
#[cfg(fuzzing)]
fn main() {
    loop {
		fuzz!(|data: &[u8]| {
            fuzz_one_input(data);
	});
	}
}

#[cfg(not(fuzzing))]
fn main() -> io::Result<()> {
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;
    let data = buffer.as_bytes();
    println!("Trying data: {:?}", data);
    fuzz_one_input(data);
    Ok(())
}


