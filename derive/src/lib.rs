// Copyright 2017-2018 Parity Technologies
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

//! Derives serialization and deserialization codec for complex structs for simple marshalling.

#![recursion_limit = "128"]
extern crate proc_macro;
use proc_macro2;

#[macro_use]
extern crate syn;

#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use proc_macro2::Span;
use syn::{Data, Field, Fields, DeriveInput, Ident, parse::Error, spanned::Spanned};
use proc_macro_crate::crate_name;

use std::env;

mod decode;
mod encode;
mod utils;
mod trait_bounds;

/// Include the `parity-scale-codec` crate under a known name (`_parity_scale_codec`).
fn include_parity_scale_codec_crate() -> proc_macro2::TokenStream {
	// This "hack" is required for the tests.
	if env::var("CARGO_PKG_NAME").unwrap() == "parity-scale-codec" {
		quote!( extern crate parity_scale_codec as _parity_scale_codec; )
	} else {
		match crate_name("parity-scale-codec") {
			Ok(parity_codec_crate) => {
				let ident = Ident::new(&parity_codec_crate, Span::call_site());
				quote!( extern crate #ident as _parity_scale_codec; )
			},
			Err(e) => Error::new(Span::call_site(), &e).to_compile_error(),
		}
	}
}

/// Wraps the impl block in a "dummy const"
fn wrap_with_dummy_const(impl_block: proc_macro2::TokenStream) -> TokenStream {
	let parity_codec_crate = include_parity_scale_codec_crate();

	let generated = quote! {
		const _: () = {
			#[allow(unknown_lints)]
			#[cfg_attr(feature = "cargo-clippy", allow(useless_attribute))]
			#[allow(rust_2018_idioms)]
			#parity_codec_crate
			#impl_block
		};
	};

	generated.into()
}

#[proc_macro_derive(Encode, attributes(codec))]
pub fn encode_derive(input: TokenStream) -> TokenStream {
	let mut input: DeriveInput = match syn::parse(input) {
		Ok(input) => input,
		Err(e) => return e.to_compile_error().into(),
	};
	if let Some(span) = utils::get_skip(&input.attrs) {
		return Error::new(span, "invalid attribute `skip` on root input")
			.to_compile_error().into();
	}

	if let Err(e) = trait_bounds::add(
		&input.ident,
		&mut input.generics,
		&input.data,
		parse_quote!(_parity_scale_codec::Encode),
		None,
		utils::get_dumb_trait_bound(&input.attrs),
	) {
		return e.to_compile_error().into();
	}

	let name = &input.ident;
	let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

	let encode_impl = encode::quote(&input.data, name);

	let impl_block = quote! {
		impl #impl_generics _parity_scale_codec::Encode for #name #ty_generics #where_clause {
			#encode_impl
		}

		impl #impl_generics _parity_scale_codec::EncodeLike for #name #ty_generics #where_clause {}
	};

	wrap_with_dummy_const(impl_block)
}

#[proc_macro_derive(Decode, attributes(codec))]
pub fn decode_derive(input: TokenStream) -> TokenStream {
	let mut input: DeriveInput = match syn::parse(input) {
		Ok(input) => input,
		Err(e) => return e.to_compile_error().into(),
	};
	if let Some(span) = utils::get_skip(&input.attrs) {
		return Error::new(span, "invalid attribute `skip` on root input")
			.to_compile_error().into();
	}

	if let Err(e) = trait_bounds::add(
		&input.ident,
		&mut input.generics,
		&input.data,
		parse_quote!(_parity_scale_codec::Decode),
		Some(parse_quote!(Default)),
		utils::get_dumb_trait_bound(&input.attrs),
	) {
		return e.to_compile_error().into();
	}

	let name = &input.ident;
	let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

	let input_ = quote!(input);
	let decoding = decode::quote(&input.data, name, &input_);

	let impl_block = quote! {
		impl #impl_generics _parity_scale_codec::Decode for #name #ty_generics #where_clause {
			fn decode<DecIn: _parity_scale_codec::Input>(
				#input_: &mut DecIn
			) -> core::result::Result<Self, _parity_scale_codec::Error> {
				#decoding
			}
		}
	};

	wrap_with_dummy_const(impl_block)
}

#[proc_macro_derive(CompactAs, attributes(codec))]
pub fn compact_as_derive(input: TokenStream) -> TokenStream {
	let mut input: DeriveInput = match syn::parse(input) {
		Ok(input) => input,
		Err(e) => return e.to_compile_error().into(),
	};

	if let Err(e) = trait_bounds::add(
		&input.ident,
		&mut input.generics,
		&input.data,
		parse_quote!(_parity_scale_codec::CompactAs),
		None,
		utils::get_dumb_trait_bound(&input.attrs),
	) {
		return e.to_compile_error().into();
	}

	let name = &input.ident;
	let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

	fn val_or_default(field: &Field) -> proc_macro2::TokenStream {
		let skip = utils::get_skip(&field.attrs).is_some();
		if skip {
			quote_spanned!(field.span()=> Default::default())
		} else {
			quote_spanned!(field.span()=> x)
		}
	}

	let (inner_ty, inner_field, constructor) = match input.data {
		Data::Struct(ref data) => {
			match data.fields {
				Fields::Named(ref fields) if utils::filter_skip_named(fields).count() == 1 => {
					let recurse = fields.named.iter().map(|f| {
						let name_ident = &f.ident;
						let val_or_default = val_or_default(&f);
						quote_spanned!(f.span()=> #name_ident: #val_or_default)
					});
					let field = utils::filter_skip_named(fields).next().expect("Exactly one field");
					let field_name = &field.ident;
					let constructor = quote!( #name { #( #recurse, )* });
					(&field.ty, quote!(&self.#field_name), constructor)
				},
				Fields::Unnamed(ref fields) if utils::filter_skip_unnamed(fields).count() == 1 => {
					let recurse = fields.unnamed.iter().enumerate().map(|(_, f) | {
						let val_or_default = val_or_default(&f);
						quote_spanned!(f.span()=> #val_or_default)
					});
					let (id, field) = utils::filter_skip_unnamed(fields).next().expect("Exactly one field");
					let id = syn::Index::from(id);
					let constructor = quote!( #name(#( #recurse, )*));
					(&field.ty, quote!(&self.#id), constructor)
				},
				_ => {
					return Error::new(
						data.fields.span(),
						"Only structs with a single non-skipped field can derive CompactAs"
					).to_compile_error().into();
				},
			}
		},
		Data::Enum(syn::DataEnum { enum_token: syn::token::Enum { span }, .. }) |
		Data::Union(syn::DataUnion { union_token: syn::token::Union { span }, .. }) => {
			return Error::new(span, "Only structs can derive CompactAs").to_compile_error().into();
		},
	};

	let impl_block = quote! {
		impl #impl_generics _parity_scale_codec::CompactAs for #name #ty_generics #where_clause {
			type As = #inner_ty;
			fn encode_as(&self) -> &#inner_ty {
				#inner_field
			}
			fn decode_from(x: #inner_ty) -> #name #ty_generics {
				#constructor
			}
		}

		impl #impl_generics From<_parity_scale_codec::Compact<#name #ty_generics>> for #name #ty_generics #where_clause {
			fn from(x: _parity_scale_codec::Compact<#name #ty_generics>) -> #name #ty_generics {
				x.0
			}
		}
	};

	wrap_with_dummy_const(impl_block)
}
