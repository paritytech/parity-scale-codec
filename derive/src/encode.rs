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

use proc_macro2::{Ident, Span, TokenStream};
use syn::{
	punctuated::Punctuated,
	spanned::Spanned,
	token::Comma,
	Data, Field, Fields, Error,
};

use crate::utils;

type FieldsList = Punctuated<Field, Comma>;

// Encode a signle field by using using_encoded, must not have skip attribute
fn encode_single_field(
	field: &Field,
	field_name: TokenStream,
	crate_ident: &TokenStream,
) -> TokenStream {
	let encoded_as = utils::get_encoded_as_type(field);
	let compact = utils::is_compact(field);

	if utils::should_skip(&field.attrs) {
		return Error::new(
			Span::call_site(),
			"Internal error: cannot encode single field optimisation if skipped"
		).to_compile_error();
	}

	if encoded_as.is_some() && compact {
		return Error::new(
			Span::call_site(),
			"`encoded_as` and `compact` can not be used at the same time!"
		).to_compile_error();
	}

	let final_field_variable = if compact {
		let field_type = &field.ty;
		quote_spanned! {
			field.span() => {
				<<#field_type as #crate_ident::HasCompact>::Type as
				#crate_ident::EncodeAsRef<'_, #field_type>>::RefType::from(#field_name)
			}
		}
	} else if let Some(encoded_as) = encoded_as {
		let field_type = &field.ty;
		quote_spanned! {
			field.span() => {
				<#encoded_as as
				#crate_ident::EncodeAsRef<'_, #field_type>>::RefType::from(#field_name)
			}
		}
	} else {
		quote_spanned! { field.span() =>
			#field_name
		}
	};

	// This may have different hygiene than the field span
	let i_self = quote! { self };

	quote_spanned! { field.span() =>
			fn encode_to<__CodecOutputEdqy: #crate_ident::Output + ?::core::marker::Sized>(
				&#i_self,
				__codec_dest_edqy: &mut __CodecOutputEdqy
			) {
				#crate_ident::Encode::encode_to(&#final_field_variable, __codec_dest_edqy)
			}

			fn encode(&#i_self) -> #crate_ident::alloc::vec::Vec<::core::primitive::u8> {
				#crate_ident::Encode::encode(&#final_field_variable)
			}

			fn using_encoded<R, F: ::core::ops::FnOnce(&[::core::primitive::u8]) -> R>(&#i_self, f: F) -> R {
				#crate_ident::Encode::using_encoded(&#final_field_variable, f)
			}
	}
}

fn encode_fields<F>(
	dest: &TokenStream,
	fields: &FieldsList,
	field_name: F,
	crate_ident: &TokenStream,
) -> TokenStream where
	F: Fn(usize, &Option<Ident>) -> TokenStream,
{
	let recurse = fields.iter().enumerate().map(|(i, f)| {
		let field = field_name(i, &f.ident);
		let encoded_as = utils::get_encoded_as_type(f);
		let compact = utils::is_compact(f);
		let skip = utils::should_skip(&f.attrs);

		if encoded_as.is_some() as u8 + compact as u8 + skip as u8 > 1 {
			return Error::new(
				f.span(),
				"`encoded_as`, `compact` and `skip` can only be used one at a time!"
			).to_compile_error();
		}

		// Based on the seen attribute, we generate the code that encodes the field.
		// We call `push` from the `Output` trait on `dest`.
		if compact {
			let field_type = &f.ty;
			quote_spanned! {
				f.span() => {
					#crate_ident::Encode::encode_to(
						&<
							<#field_type as #crate_ident::HasCompact>::Type as
							#crate_ident::EncodeAsRef<'_, #field_type>
						>::RefType::from(#field),
						#dest,
					);
				}
			}
		} else if let Some(encoded_as) = encoded_as {
			let field_type = &f.ty;
			quote_spanned! {
				f.span() => {
					#crate_ident::Encode::encode_to(
						&<
							#encoded_as as
							#crate_ident::EncodeAsRef<'_, #field_type>
						>::RefType::from(#field),
						#dest,
					);
				}
			}
		} else if skip {
			quote! {
				let _ = #field;
			}
		} else {
			quote_spanned! { f.span() =>
				#crate_ident::Encode::encode_to(#field, #dest);
			}
		}
	});

	quote! {
		#( #recurse )*
	}
}

fn try_impl_encode_single_field_optimisation(data: &Data, crate_ident: &TokenStream) -> Option<TokenStream> {
	match *data {
		Data::Struct(ref data) => {
			match data.fields {
				Fields::Named(ref fields) if utils::filter_skip_named(fields).count() == 1 => {
					let field = utils::filter_skip_named(fields).next().unwrap();
					let name = &field.ident;
					Some(encode_single_field(
						field,
						quote!(&self.#name),
						crate_ident,
					))
				},
				Fields::Unnamed(ref fields) if utils::filter_skip_unnamed(fields).count() == 1 => {
					let (id, field) = utils::filter_skip_unnamed(fields).next().unwrap();
					let id = syn::Index::from(id);

					Some(encode_single_field(
						field,
						quote!(&self.#id),
						crate_ident,
					))
				},
				_ => None,
			}
		},
		_ => None,
	}
}

fn impl_encode(data: &Data, type_name: &Ident, crate_ident: &TokenStream) -> TokenStream {
	let self_ = quote!(self);
	let dest = &quote!(__codec_dest_edqy);
	let encoding = match *data {
		Data::Struct(ref data) => {
			match data.fields {
				Fields::Named(ref fields) => encode_fields(
					dest,
					&fields.named,
					|_, name| quote!(&#self_.#name),
					crate_ident,
				),
				Fields::Unnamed(ref fields) => encode_fields(
					dest,
					&fields.unnamed,
					|i, _| {
						let i = syn::Index::from(i);
						quote!(&#self_.#i)
					},
					crate_ident,
				),
				Fields::Unit => quote!(),
			}
		},
		Data::Enum(ref data) => {
			let data_variants = || data.variants.iter().filter(|variant| !utils::should_skip(&variant.attrs));

			if data_variants().count() > 256 {
				return Error::new(
					data.variants.span(),
					"Currently only enums with at most 256 variants are encodable."
				).to_compile_error();
			}

			// If the enum has no variants, we don't need to encode anything.
			if data_variants().count() == 0 {
				return quote!();
			}

			let recurse = data_variants().enumerate().map(|(i, f)| {
				let name = &f.ident;
				let index = utils::variant_index(f, i);

				match f.fields {
					Fields::Named(ref fields) => {
						let field_name = |_, ident: &Option<Ident>| quote!(#ident);
						let names = fields.named
							.iter()
							.enumerate()
							.map(|(i, f)| field_name(i, &f.ident));

						let encode_fields = encode_fields(
							dest,
							&fields.named,
							|a, b| field_name(a, b),
							crate_ident,
						);

						quote_spanned! { f.span() =>
							#type_name :: #name { #( ref #names, )* } => {
								#dest.push_byte(#index as ::core::primitive::u8);
								#encode_fields
							}
						}
					},
					Fields::Unnamed(ref fields) => {
						let field_name = |i, _: &Option<Ident>| {
							let data = stringify(i as u8);
							let ident = from_utf8(&data).expect("We never go beyond ASCII");
							let ident = Ident::new(ident, Span::call_site());
							quote!(#ident)
						};
						let names = fields.unnamed
							.iter()
							.enumerate()
							.map(|(i, f)| field_name(i, &f.ident));

						let encode_fields = encode_fields(
							dest,
							&fields.unnamed,
							|a, b| field_name(a, b),
							crate_ident,
						);

						quote_spanned! { f.span() =>
							#type_name :: #name ( #( ref #names, )* ) => {
								#dest.push_byte(#index as ::core::primitive::u8);
								#encode_fields
							}
						}
					},
					Fields::Unit => {
						quote_spanned! { f.span() =>
							#type_name :: #name => {
								#dest.push_byte(#index as ::core::primitive::u8);
							}
						}
					},
				}
			});

			quote! {
				match *#self_ {
					#( #recurse )*,
					_ => (),
				}
			}
		},
		Data::Union(ref data) => Error::new(
			data.union_token.span(),
			"Union types are not supported."
		).to_compile_error(),
	};
	quote! {
		fn encode_to<__CodecOutputEdqy: #crate_ident::Output + ?::core::marker::Sized>(
			&#self_,
			#dest: &mut __CodecOutputEdqy
		) {
			#encoding
		}
	}
}

pub fn quote(data: &Data, type_name: &Ident, crate_ident: &TokenStream) -> TokenStream {
	if let Some(implementation) = try_impl_encode_single_field_optimisation(data, crate_ident) {
		implementation
	} else {
		impl_encode(data, type_name, crate_ident)
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
