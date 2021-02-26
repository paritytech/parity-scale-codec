// Copyright 2021 Parity Technologies
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

//! Test for type inference issue in decode.

#[cfg(not(feature = "derive"))]
use parity_scale_codec_derive::Decode;
use parity_scale_codec::Decode;

pub trait Trait {
	type Value;
	type AccountId: Decode;
}

#[derive(Decode)]
pub enum A<T: Trait> {
	_C(
		(T::AccountId, T::AccountId),
		Vec<(T::Value, T::Value)>,
	),
}

#[derive(Decode)]
pub struct B<T: Trait>((T::AccountId, T::AccountId), Vec<(T::Value, T::Value)>);
