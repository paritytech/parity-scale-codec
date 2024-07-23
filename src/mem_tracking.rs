// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::Decode;
use impl_trait_for_tuples::impl_for_tuples;

/// Marker trait used for identifying types that call the mem tracking hooks exposed by `Input`
/// while decoding.
pub trait DecodeWithMemTracking: Decode {}

#[impl_for_tuples(18)]
impl DecodeWithMemTracking for Tuple {}
