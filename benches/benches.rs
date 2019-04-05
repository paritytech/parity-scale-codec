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

extern crate test;

use parity_codec::*;

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
