// Copyright 2019 Parity Technologies
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::time::Duration;

use bitvec::vec::BitVec;
use criterion::{Criterion, black_box, Bencher, criterion_group, criterion_main};
use parity_scale_codec::*;
use parity_scale_codec_derive::{Encode, Decode};

fn array_vec_write_u128(b: &mut Bencher) {
	b.iter(|| {
		for b in 0..black_box(1_000_000) {
			let a = 0xffff_ffff_ffff_ffff_ffff_u128;
			Compact(a ^ b).using_encoded(|x| {
				black_box(x).len()
			});
		}
	});
}

fn test_vec<F: Fn(&mut Vec<u8>, &[u8])>(b: &mut Bencher, f: F) {
	let f = black_box(f);
	let x = black_box([0xff; 10240]);

	b.iter(|| {
		for _b in 0..black_box(10_000) {
			let mut vec = Vec::<u8>::new();
			f(&mut vec, &x);
		}
	});
}

fn vec_write_as_output(b: &mut Bencher) {
	test_vec(b, |vec, a| {
		Output::write(vec, a);
	});
}

fn vec_extend(b: &mut Bencher) {
	test_vec(b, |vec, a| {
		vec.extend(a);
	});
}

fn vec_extend_from_slice(b: &mut Bencher) {
	test_vec(b, |vec, a| {
		vec.extend_from_slice(a);
	});
}

#[derive(Encode, Decode)]
enum Event {
	ComplexEvent(Vec<u8>, u32, i32, u128, i8),
}

fn vec_append_with_decode_and_encode(b: &mut Bencher) {
	let data = b"PCX";

	b.iter(|| {
		let mut encoded_events_vec = Vec::new();
		for _ in 0..1000 {
			let mut events = Vec::<Event>::decode(&mut &encoded_events_vec[..])
				.unwrap_or_default();

			events.push(Event::ComplexEvent(data.to_vec(), 4, 5, 6, 9));

			encoded_events_vec = events.encode();
		}
	})
}

fn vec_append_with_encode_append(b: &mut Bencher) {
	let data = b"PCX";

	b.iter(|| {
		let mut encoded_events_vec;

		let events = vec![Event::ComplexEvent(data.to_vec(), 4, 5, 6, 9)];
		encoded_events_vec = events.encode();

		for _ in 1..1000 {
			encoded_events_vec = <Vec<Event> as EncodeAppend>::append_or_new(
				encoded_events_vec,
				&[Event::ComplexEvent(data.to_vec(), 4, 5, 6, 9)],
			).unwrap();
		}
	});
}

fn encode_decode_vec_u8(c: &mut Criterion) {
	c.bench_function_over_inputs("vec_u8_encode - Vec<u8>", |b, &vec_size| {
		let vec: Vec<u8> = (0..=255u8)
			.cycle()
			.take(vec_size)
			.collect();

		let vec = black_box(vec);
		b.iter(|| vec.encode())
	}, vec![1, 2, 5, 32, 1024]);

	c.bench_function_over_inputs("vec_u8_decode - Vec<u8>", |b, &vec_size| {
		let vec: Vec<u8> = (0..=255u8)
			.cycle()
			.take(vec_size)
			.collect();

		let vec = vec.encode();

		let vec = black_box(vec);
		b.iter(|| {
			let _: Vec<u8> = Decode::decode(&mut &vec[..]).unwrap();
		})
	}, vec![1, 2, 5, 32, 1024]);
}

fn bench_fn(c: &mut Criterion) {
	c.bench_function("vec_write_as_output", vec_write_as_output);
	c.bench_function("vec_extend", vec_extend);
	c.bench_function("vec_extend_from_slice", vec_extend_from_slice);
	c.bench_function("vec_append_with_decode_and_encode", vec_append_with_decode_and_encode);
	c.bench_function("vec_append_with_encode_append", vec_append_with_encode_append);
	c.bench_function("array_vec_write_u128", array_vec_write_u128);
}

fn encode_decode_bitvec_u8(c: &mut Criterion) {
	c.bench_function_over_inputs("bitvec_u8_encode - BitVec<u8>", |b, &size| {
		let vec: BitVec = [true, false]
			.iter()
			.cloned()
			.cycle()
			.take(size)
			.collect();

		let vec = black_box(vec);
		b.iter(|| vec.encode())
	}, vec![1, 2, 5, 32, 1024]);

	c.bench_function_over_inputs("bitvec_u8_decode - BitVec<u8>", |b, &size| {
		let vec: BitVec = [true, false]
			.iter()
			.cloned()
			.cycle()
			.take(size)
			.collect();

		let vec = vec.encode();

		let vec = black_box(vec);
		b.iter(|| {
			let _: BitVec = Decode::decode(&mut &vec[..]).unwrap();
		})
	}, vec![1, 2, 5, 32, 1024]);
}

criterion_group!{
	name = benches;
	config = Criterion::default().warm_up_time(Duration::from_millis(500)).without_plots();
	targets = encode_decode_vec_u8, bench_fn, encode_decode_bitvec_u8
}
criterion_main!(benches);
