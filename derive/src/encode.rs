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

use std::str::from_utf8;

use proc_macro2::{Span, TokenStream};
use syn::{
	Data, Field, Fields, Ident, Index,
	punctuated::Punctuated,
	spanned::Spanned,
	token::Comma,
	Error,
};
use crate::utils;

type FieldsList = Punctuated<Field, Comma>;

fn encode_fields<F>(
	dest: &TokenStream,
	fields: &FieldsList,
	field_name: F,
) -> TokenStream where
	F: Fn(usize, &Option<Ident>) -> TokenStream,
{
	let recurse = fields.iter().enumerate().map(|(i, f)| {
		let field = field_name(i, &f.ident);
		let encoded_as = utils::get_encoded_as_type(f);
		let compact = utils::get_enable_compact(f);

		if encoded_as.is_some() && compact {
			return Error::new(
				Span::call_site(),
				"`encoded_as` and `compact` can not be used at the same time!"
			).to_compile_error();
		}

		// Based on the seen attribute, we generate the code that encodes the field.
		// We call `push` from the `Output` trait on `dest`.
		if compact {
			let field_type = &f.ty;
			quote_spanned! {
				f.span() => {
					#dest.push(
						&<<#field_type as _parity_codec::HasCompact>::Type as
							_parity_codec::EncodeAsRef<'_, #field_type>>::RefType::from(#field)
					);
				}
			}
		} else if let Some(encoded_as) = encoded_as {
			let field_type = &f.ty;
			quote_spanned! {
				f.span() => {
					#dest.push(
						&<#encoded_as as
							_parity_codec::EncodeAsRef<'_, #field_type>>::RefType::from(#field)
					);
				}
			}
		} else {
			quote_spanned! { f.span() =>
					#dest.push(#field);
			}
		}
	});

	quote! {
		#( #recurse )*
	}
}

pub fn quote(data: &Data, type_name: &Ident, self_: &TokenStream, dest: &TokenStream) -> TokenStream {
	let call_site = Span::call_site();
	match *data {
		Data::Struct(ref data) => {
			match data.fields {
				Fields::Named(ref fields) => encode_fields(
					dest,
					&fields.named,
					|_, name| quote_spanned!(call_site => &#self_.#name),
				),
				Fields::Unnamed(ref fields) => encode_fields(
					dest,
					&fields.unnamed,
					|i, _| {
						let index = Index { index: i as u32, span: call_site };
						quote_spanned!(call_site => &#self_.#index)
					},
				),
				Fields::Unit => quote_spanned! { call_site =>
					drop(#dest);
				},
			}
		},
		Data::Enum(ref data) => {
			if data.variants.len() > 256 {
				return Error::new(
					Span::call_site(),
					"Currently only enums with at most 256 variants are encodable."
				).to_compile_error();
			}

			let recurse = data.variants.iter().enumerate().map(|(i, f)| {
				let name = &f.ident;
				let index = utils::index(f, i);

				match f.fields {
					Fields::Named(ref fields) => {
						let field_name = |_, ident: &Option<Ident>| quote_spanned!(call_site => #ident);
						let names = fields.named
							.iter()
							.enumerate()
							.map(|(i, f)| field_name(i, &f.ident));

						let encode_fields = encode_fields(
							dest,
							&fields.named,
							|a, b| field_name(a, b),
						);

						quote_spanned! { f.span() =>
							#type_name :: #name { #( ref #names, )* } => {
								#dest.push_byte(#index as u8);
								#encode_fields
							}
						}
					},
					Fields::Unnamed(ref fields) => {
						let field_name = |i, _: &Option<Ident>| {
							let data = stringify(i as u8);
							let ident = from_utf8(&data).expect("We never go beyond ASCII");
							let ident = Ident::new(ident, call_site);
							quote_spanned!(call_site => #ident)
						};
						let names = fields.unnamed
							.iter()
							.enumerate()
							.map(|(i, f)| field_name(i, &f.ident));

						let encode_fields = encode_fields(
							dest,
							&fields.unnamed,
							|a, b| field_name(a, b),
						);

						quote_spanned! { f.span() =>
							#type_name :: #name ( #( ref #names, )* ) => {
								#dest.push_byte(#index as u8);
								#encode_fields
							}
						}
					},
					Fields::Unit => {
						quote_spanned! { f.span() =>
							#type_name :: #name => {
								#dest.push_byte(#index as u8);
							}
						}
					},
				}
			});

			quote! {
				match *#self_ {
					#( #recurse )*,
				}
			}
		},
		Data::Union(_) => Error::new(Span::call_site(), "Union types are not supported.").to_compile_error(),
	}
}
pub fn stringify(id: u8) -> [u8; 2] {
	const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
	let len = CHARS.len() as u8;
	let symbol = |id: u8| CHARS[(id % len) as usize];
	let a = symbol(id);
	let b = symbol(id / len);

	[a, b]
}
