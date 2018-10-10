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

//! Derives serialization and deserialization codec for complex structs for simple marshalling.

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), feature(core_intrinsics))]
#![cfg_attr(not(feature = "std"), feature(alloc))]

#[cfg(not(feature = "std"))]
extern crate alloc;

extern crate proc_macro;
extern crate proc_macro2;

#[macro_use]
extern crate syn;

#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use proc_macro2::Span;
use syn::{DeriveInput, Generics, GenericParam, Ident};

#[cfg(not(feature = "std"))]
use alloc::string::ToString;

mod decode;
mod encode;

const ENCODE_ERR: &str = "derive(Encode) failed";

#[proc_macro_derive(Encode, attributes(codec))]
pub fn encode_derive(input: TokenStream) -> TokenStream {
	let input: DeriveInput = syn::parse(input).expect(ENCODE_ERR);
	let name = &input.ident;

	let generics = add_trait_bounds(input.generics, parse_quote!(_parity_codec::Encode));
	let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

	let self_ = quote!(self);
	let dest_ = quote!(dest);
	let encoding = encode::quote(&input.data, name, &self_, &dest_);

	let impl_block = quote! {
		impl #impl_generics _parity_codec::Encode for #name #ty_generics #where_clause {
			fn encode_to<EncOut: _parity_codec::Output>(&#self_, #dest_: &mut EncOut) {
				#encoding
			}
		}
	};

	let mut new_name = "_IMPL_ENCODE_FOR_".to_string();
	new_name.push_str(name.to_string().trim_left_matches("r#"));
	let dummy_const = Ident::new(&new_name, Span::call_site());

	let generated = quote! {
		#[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
		const #dummy_const: () = {
			#[allow(unknown_lints)]
			#[cfg_attr(feature = "cargo-clippy", allow(useless_attribute))]
			#[allow(rust_2018_idioms)]
			extern crate parity_codec as _parity_codec;
			#impl_block
		};
	};

	generated.into()
}

#[proc_macro_derive(Decode, attributes(codec))]
pub fn decode_derive(input: TokenStream) -> TokenStream {
	let input: DeriveInput = syn::parse(input).expect(ENCODE_ERR);
	let name = &input.ident;

	let generics = add_trait_bounds(input.generics, parse_quote!(_parity_codec::Decode));
	let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

	let input_ = quote!(input);
	let decoding = decode::quote(&input.data, name, &input_);

	let impl_block = quote! {
		impl #impl_generics _parity_codec::Decode for #name #ty_generics #where_clause {
			fn decode<DecIn: _parity_codec::Input>(#input_: &mut DecIn) -> Option<Self> {
				#decoding
			}
		}
	};

	let mut new_name = "_IMPL_DECODE_FOR_".to_string();
	new_name.push_str(name.to_string().trim_left_matches("r#"));
	let dummy_const = Ident::new(&new_name, Span::call_site());

	let generated = quote! {
		#[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
		const #dummy_const: () = {
			#[allow(unknown_lints)]
			#[cfg_attr(feature = "cargo-clippy", allow(useless_attribute))]
			#[allow(rust_2018_idioms)]
			extern crate parity_codec as _parity_codec;
			#impl_block
		};
	};

	generated.into()
}

fn add_trait_bounds(mut generics: Generics, bounds: syn::TypeParamBound) -> Generics {
	for param in &mut generics.params {
		if let GenericParam::Type(ref mut type_param) = *param {
			type_param.bounds.push(bounds.clone());
		}
	}
	generics
}

fn index(v: &syn::Variant, i: usize) -> proc_macro2::TokenStream {
	// look for an index in attributes
	let index = v.attrs.iter().filter_map(|attr| {
		let pair = attr.path.segments.first()?;
		let seg = pair.value();

		if seg.ident == Ident::new("codec", seg.ident.span()) {
			assert_eq!(attr.path.segments.len(), 1);

			let meta = attr.interpret_meta();
			if let Some(syn::Meta::List(ref l)) = meta {
				if let syn::NestedMeta::Meta(syn::Meta::NameValue(ref nv)) = l.nested.last().unwrap().value() {
					assert_eq!(nv.ident, Ident::new("index", nv.ident.span()));
					if let syn::Lit::Str(ref s) = nv.lit {
						let byte: u8 = s.value().parse().expect("Numeric index expected.");
						return Some(byte)
					}
					panic!("Invalid syntax for `codec` attribute: Expected string literal.")
				}
			}
			panic!("Invalid syntax for `codec` attribute: Expected `name = value` pair.")
		} else {
			None
		}
	}).next();

	// then fallback to discriminant or just index
	index.map(|i| quote! { #i })
		.unwrap_or_else(|| v.discriminant
			.as_ref()
			.map(|&(_, ref expr)| quote! { #expr })
			.unwrap_or_else(|| quote! { #i })
		)
}

fn get_encode_type(field_entry: &syn::Field) -> Option<String> {
	// look for an encode_as in attributes
	let encoder_type = field_entry.attrs.iter().filter_map(|attr| {
		let pair = attr.path.segments.first()?;
		let seg = pair.value();

		if seg.ident == Ident::new("codec", seg.ident.span()) {
			assert_eq!(attr.path.segments.len(), 1);

			let meta = attr.interpret_meta();
			if let Some(syn::Meta::List(ref l)) = meta {
				if let syn::NestedMeta::Meta(syn::Meta::NameValue(ref nv)) = l.nested.last().unwrap().value() {
					assert_eq!(nv.ident, Ident::new("encode_as", nv.ident.span()));
					if let syn::Lit::Str(ref s) = nv.lit {
						let encoding: String = s.value();
						return Some(encoding)
					}
					panic!("Invalid syntax for `codec` attribute: Expected string literal.")
				}
			}
			panic!("Invalid syntax for `codec` attribute: Expected `name = value` pair.")
		} else {
			None
		}
	}).next();

	encoder_type
}