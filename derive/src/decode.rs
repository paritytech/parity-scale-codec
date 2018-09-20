// Copyright 2017, 2018 Parity Technologies
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

use proc_macro2::{Span, TokenStream, Ident};
use syn::{
	Data, Fields,
	spanned::Spanned,
};

pub fn quote(data: &Data, type_name: &Ident, input: &TokenStream) -> TokenStream {
	let call_site = Span::call_site();
	match *data {
		Data::Struct(ref data) => match data.fields {
			Fields::Named(_) | Fields::Unnamed(_) => create_instance(
				call_site,
				quote! { #type_name },
				input,
				&data.fields,
			),
			Fields::Unit => {
				quote_spanned! {call_site =>
					drop(#input);
					Some(#type_name)
				}
			},
		},
		Data::Enum(ref data) => {
			assert!(data.variants.len() < 256, "Currently only enums with at most 256 variants are encodable.");

			let recurse = data.variants.iter().enumerate().map(|(i, v)| {
				let name = &v.ident;
				let index = super::index(v, i);

				let create = create_instance(
					call_site,
					quote! { #type_name :: #name },
					input,
					&v.fields,
				);

				quote_spanned! { v.span() =>
					x if x == #index as u8 => {
						#create
					},
				}
			});

			quote! {
				match #input.read_byte()? {
					#( #recurse )*
					_ => None,
				}

			}

		},
		Data::Union(_) => panic!("Union types are not supported."),
	}
}

fn create_instance(call_site: Span, name: TokenStream, input: &TokenStream, fields: &Fields) -> TokenStream {
	match *fields {
		Fields::Named(ref fields) => {
			let recurse = fields.named.iter().map(|f| {
				let name = &f.ident;
				let field = quote_spanned!(call_site => #name);

				quote_spanned! { f.span() =>
					#field: _parity_codec::Decode::decode(#input)?
				}
			});

			quote_spanned! {call_site =>
				Some(#name {
					#( #recurse, )*
				})
			}
		},
		Fields::Unnamed(ref fields) => {
			let recurse = fields.unnamed.iter().map(|f| {
				quote_spanned! { f.span() =>
					_parity_codec::Decode::decode(#input)?
				}
			});

			quote_spanned! {call_site =>
				Some(#name (
					#( #recurse, )*
				))
			}
		},
		Fields::Unit => {
			quote_spanned! {call_site =>
				Some(#name)
			}
		},
	}
}
