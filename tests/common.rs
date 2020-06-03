// Copyright 2020 Parity Technologies
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

use parity_scale_codec::Decode;

/// Assert Decode::decode and Decode::skip works
pub fn assert_decode<T>(mut encoded: &[u8], res: T) where
	T: core::fmt::Debug + Decode + PartialEq,
{
	assert_eq!(Decode::decode(&mut encoded.clone()), Ok(res));
	assert_eq!(T::skip(&mut encoded), Ok(()));
	assert!(encoded.is_empty());
}
