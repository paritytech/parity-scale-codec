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

#![feature(test)]

#[macro_use]
extern crate criterion;
extern crate test;
#[macro_use]
extern crate parity_codec_derive;

use parity_codec::{Encode, Decode};

use criterion::{Criterion};
use std::time::Duration;


#[bench]
fn array_vec_write_u128(b: &mut test::Bencher) {
	b.iter(|| {
		for b in 0..test::black_box(1_000_000) {
			let a = 0xffff_ffff_ffff_ffff_ffff_u128;
			Compact(a ^ b).using_encoded(|x| {
				test::black_box(x).len()
			});
		}
	});
}

fn test_vec<F: Fn(&mut Vec<u8>, &[u8])>(b: &mut test::Bencher, f: F) {
	let f = test::black_box(f);
	let x = test::black_box([0xff; 10240]);

	b.iter(|| {
		for _b in 0..test::black_box(10_000) {
			let mut vec = Vec::<u8>::new();
			f(&mut vec, &x);
		}
	});
}

#[bench]
fn vec_write_as_output(b: &mut test::Bencher) {
	test_vec(b, |vec, a| {
		Output::write(vec, a);
	});
}

#[bench]
fn vec_extend(b: &mut test::Bencher) {
	test_vec(b, |vec, a| {
		vec.extend(a);
	});
}

#[bench]
fn vec_extend_from_slice(b: &mut test::Bencher) {
	test_vec(b, |vec, a| {
		vec.extend_from_slice(a);
	});
}

#[bench]
fn encoding_of_large_vec_u8(b: &mut test::Bencher) {
	let mut v = vec![];
	for i in 0..256 {
		v.push(i);
	}
	for _ in 0..12 {
		v.extend(v.clone());
	}

	b.iter(|| {
		v.encode();
	})
}

#[derive(Encode, Decode)]
enum Event {
	ComplexEvent(Vec<u8>, u32, i32, u128, i8),
}

#[bench]
fn vec_append_with_decode_and_encode(b: &mut test::Bencher) {
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

#[bench]
fn vec_append_with_encode_append(b: &mut test::Bencher) {
	let data = b"PCX";

	b.iter(|| {
		let mut encoded_events_vec;

		let events = vec![Event::ComplexEvent(data.to_vec(), 4, 5, 6, 9)];
		encoded_events_vec = events.encode();

		for _ in 1..1000 {
			encoded_events_vec = <Vec::<Event> as EncodeAppend>::append(
				encoded_events_vec,
				&[Event::ComplexEvent(data.to_vec(), 4, 5, 6, 9)],
			).unwrap();
		}
	});
}

fn codec_vec_u8(c: &mut Criterion) {
	c.bench_function_over_inputs("encode - Vec<u8>", |b, &vec_size| {
		let mut vec: Vec<u8> = vec![];
		vec.resize(vec_size, 0u8);
		b.iter(|| vec.encode())
	}, vec![1, 2, 5, 32, 1024]);

	c.bench_function_over_inputs("decode - Vec<u8>", |b, &vec_size| {
		let mut vec: Vec<u8> = vec![];
		vec.resize(vec_size, 0u8);
		let vec = vec.encode();

		b.iter(|| {
			let _: Vec<u8> = Decode::decode(&mut &vec[..]).unwrap();
		})
	}, vec![1, 2, 5, 32, 1024]);
}

criterion_group!{
	name = benches;
	config = Criterion::default().warm_up_time(Duration::from_millis(500)).without_plots();
	targets = codec_vec_u8
}
criterion_main!(benches);
