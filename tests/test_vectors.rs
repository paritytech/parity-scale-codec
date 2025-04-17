use jam_codec::{Compact, Decode, Encode, Output};
use jam_codec_derive::{Decode as DecodeDerive, Encode as EncodeDerive};
use std::vec::Vec;

/// A trivial and fast shuffle used by tests.
pub fn shuffle<T>(slice: &mut [T], seed: u64) {
	let mut r = seed as usize;
	for i in (1..slice.len()).rev() {
		let j = r % (i + 1);
		slice.swap(i, j);
		r = r.wrapping_mul(6364793005) + 1;
	}
}

trait Shuffle {
	fn shuffle(self) -> Self;
}

impl<T> Shuffle for Vec<T> {
	fn shuffle(mut self) -> Self {
		let seed = self.len() as u64;
		shuffle(&mut self[..], seed);
		self
	}
}

macro_rules! seq {
    ( $( $x:expr ),* ) => {
        ( $( $x ),* )
    };
}

macro_rules! myvec {
    // Case 1: Create a vector from a list of elements
    ($($elem:expr),* $(,)?) => {
        {
            let mut vec = Vec::new();
            $(vec.push($elem);)*
            vec
        }
    };
    // Case 2: Create a vector by repeating an element
    ($elem:expr; $count:expr) => {
        {
            let mut vec = Vec::with_capacity($count);
            vec.extend(std::iter::repeat($elem).take($count));
            vec
        }
    };
    ($start:expr => $end:expr) => {
        {
            let mut v = Vec::new();
            for i in $start..$end {
                v.push(i);
            }
			v
        }
    };
}

#[derive(EncodeDerive, DecodeDerive, std::fmt::Debug, PartialEq)]
enum TestEnum<T> {
	Dummy,
	Foo(T),
	Bar([T; 8]),
}

#[derive(Default)]
struct TestHarness {
	dump: Vec<u8>,
}

impl TestHarness {
	fn new() -> Self {
		Self::default()
	}

	fn process<T: Encode + Decode + std::fmt::Debug + PartialEq>(&mut self, v: T) {
		println!("-------------------------------");
		println!("[{:#?}]", std::any::type_name::<T>());
		let buf = v.encode();
		println!("{}", hex::encode(&buf));
		self.dump.extend_from_slice(&buf[..]);
		let d = T::decode(&mut &buf[..]).unwrap();
		assert_eq!(v, d);
	}

	fn process_bit_string_vl(&mut self, bit_string: &str) {
		let buf = Compact(bit_string.len() as u64).encode();
		print!("{}", hex::encode(&buf));
		self.dump.extend_from_slice(&buf[..]);
		self.process_bit_string_fl(bit_string);
	}

	// Bits represented by the bits string are processed left to right in groups of 8 bits.
	// Examples:
	// * "1101" ≡ 0x0b
	// * "10001011 01101" ≡

	fn process_bit_string_fl(&mut self, bit_string: &str) {
		let mut buf = Vec::with_capacity((bit_string.len() + 7) / 8);
		bit_string.as_bytes().chunks(8).for_each(|chunk| {
			let octet = chunk.iter().enumerate().fold(0, |octet, (i, &bit)| {
				let b = (bit == b'1') as u8;
				octet | (b << i)
			});
			buf.push_byte(octet);
		});
		println!("{}", hex::encode(&buf));
		self.dump.extend_from_slice(&buf[..]);
	}
}

#[cfg(feature = "dump-test-vectors")]
impl Drop for TestHarness {
	fn drop(&mut self) {
		use std::{fs::File, io::Write};
		const DUMP_FILE: &str = "vectors.bin";
		let mut file = File::create(DUMP_FILE).unwrap();
		file.write_all(&self.dump).unwrap()
	}
}

#[test]
fn make_vectors() {
	// Sequences in different flavors
	// NOTE: In the end everything can be once of these three types:
	// - A primitive integer
	// - A non-uniform fixed length sequence (aka tuple / struct)
	// - A uniform fixed length sequence (aka an array)
	// - A uniform variable length sequence (aka a vector)
	// - An "choice" (aka an enum)

	let mut t = TestHarness::new();

	// Non-uniform fixed length sequence

	t.process(seq!(0xf1_u8, 0x1234_u16, 0xFF00cc11_u32, 0x1231092319023131_u64));

	#[rustfmt::skip]
	t.process(seq!{
		0xf1_u8,
		seq!{
			seq!{
				0x1234_u16,
				0xFF00cc11_u32
			},
			seq!{
				0x1231092319023131_u64,
				seq! {
					0x32_u8
				},
				3_i32
			}
		}
	});

	// Uniform fixed length sequences

	t.process([0_u8; 0]);
	t.process([(3_u8, 0x3122_u16), (8, 0x3321), (9, 0x9973)]);
	t.process(TryInto::<[u8; 16]>::try_into(myvec![0_u8 => 16].shuffle()).unwrap());

	// Uniform variable length sequences

	t.process(myvec![1_u16, 2, 3]);
	t.process(myvec!(0_u16 => 127));
	t.process(myvec!(0_u8 => 200));

	// Enumerations

	t.process(TestEnum::<u8>::Dummy);
	t.process(TestEnum::Foo(42_u8));
	t.process(TestEnum::Bar([1_u8, 2, 3, 4, 5, 6, 7, 8]));

	// Optional entries

	t.process(Option::<u16>::None);
	t.process(Some(42_u8));

	#[rustfmt::skip]
	t.process(
		myvec!(0 => 15).shuffle().iter().map(|&i|
			if i % 3 == 0 {
				Option::None
			} else {
				Option::Some(myvec![0_u8 => i as u8].shuffle())
			},
		).collect::<Vec<_>>()
	);

	#[rustfmt::skip]
	t.process(seq! {
		(Option::Some(0x1234_u16), 42_u8),
		myvec!(0 => 15).shuffle().iter().map(|&i|
			(
				i as u8,
				if i % 3 == 0 {
					Option::None
				} else {
					Option::Some(seq!(i % 5 as u16, myvec![0_u8 => i as u8].shuffle()))
				},
	 		)
		).collect::<Vec<_>>()
	});

	// A mix of the above

	#[rustfmt::skip]
	t.process(
		myvec!(0 => 10).shuffle().iter().map(|&i|
			seq!{
				i as u16,
				seq! {
					2 * i as u64,
					Some(3 * i as u8)
				}
			}
		)
		.collect::<Vec<_>>()
	);

	#[rustfmt::skip]
	t.process(seq! {
		3_u8,
		seq! {
			0x5242_u16,
			0x3312_u16
		},
		myvec!(0_u16 => 12).shuffle(),
		myvec!(0_u8 => 30).shuffle().iter().map(|&i| seq!(i as u8, i as u32)).collect::<Vec<_>>()
	});

	// Some compact values

	t.process(Compact(0_u32));
	t.process(Compact(127_u32));
	t.process(Compact(128_u32));
	t.process(Compact(129_u32));
	t.process(Compact(1023_u32));
	t.process(Compact(0x1000_u32));
	t.process(Compact(0x3fff_u32));
	t.process(Compact(0x4000_u32));
	t.process(Compact(0x4001_u32));
	t.process(Compact(0xfff1_u32));
	t.process(Compact(0x1fffff_u32));
	t.process(Compact(0x200000_u32));
	t.process(Compact(0x200001_u32));
	t.process(Compact(0xfff1ff_u32));
	t.process(Compact(0xffffffffff_u64));
	t.process(Compact(0xab1c50bbc19a_u64));

	// Bit string
	println!("------------------------");
	println!("Fixed-length bit-strings");

	t.process_bit_string_fl("0");
	t.process_bit_string_fl("000");
	t.process_bit_string_fl("1");
	t.process_bit_string_fl("1101");
	t.process_bit_string_fl("101100001001");
	t.process_bit_string_fl("100010110110100101101101");
	t.process_bit_string_fl("010100101010010101010101010110101000100101011010101011010101101101001010110101010010101010101101011010111001001000110010101010010110101001110011111110100000000010101010010111101001001111100010000001010110110101011001010101011111111110101101");

	println!("------------------------");
	println!("Variable-length bit-strings");

	t.process_bit_string_vl("0");
	t.process_bit_string_vl("000");
	t.process_bit_string_vl("1");
	t.process_bit_string_vl("1101");
	t.process_bit_string_vl("101100001001");
	t.process_bit_string_vl("100010110110100101101101");
	t.process_bit_string_vl("010100101010010101010101010110101000100101011010101011010101101101001010110101010010101010101101011010111001001000110010101010010110101001110011111110100000000010101010010111101001001111100010000001010110110101011001010101011111111110101101");
}
