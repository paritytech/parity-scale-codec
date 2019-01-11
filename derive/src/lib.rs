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

extern crate proc_macro;
extern crate proc_macro2;

#[macro_use]
extern crate syn;

#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use proc_macro2::Span;
use syn::{DeriveInput, Generics, GenericParam, Ident};

mod decode;
mod encode;
mod utils;

const ENCODE_ERR: &str = "derive(Encode) failed";

#[proc_macro_derive(Encode, attributes(codec))]
pub fn encode_derive(input: TokenStream) -> TokenStream {
	let mut input: DeriveInput = syn::parse(input).expect(ENCODE_ERR);
	patch_generics(&mut input.generics, &input.data, parse_quote!(_parity_codec::Encode));
	let name = &input.ident;

	// let generics = add_trait_bounds(input.generics, parse_quote!(_parity_codec::Encode));
	let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

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
	new_name.push_str(name.to_string().trim_start_matches("r#"));
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
	let mut input: DeriveInput = syn::parse(input).expect(ENCODE_ERR);
	patch_generics(&mut input.generics, &input.data, parse_quote!(_parity_codec::Decode));
	let name = &input.ident;

	// let generics = add_trait_bounds(input.generics, parse_quote!(_parity_codec::Decode));
	let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

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
	new_name.push_str(name.to_string().trim_start_matches("r#"));
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

fn patch_generics(generics: &mut Generics, data: &syn::Data, bound: syn::Path) {
	let types = collect_types(&data);
	if !generics.params.is_empty() && !types.is_empty() {

		if generics.where_clause.is_none() {
			generics.where_clause = Some(parse_quote!(where));
		}

		let where_clause = &mut generics.where_clause.as_mut().unwrap();

		for ty in types {
			where_clause.predicates.push(parse_quote!(#ty : #bound));
		}
		
		println!("predicates {:#?}", where_clause.predicates);
	}
}

fn collect_types(data: &syn::Data) -> Vec<syn::Type> {
	use syn::*;
	use syn::visit::Visit;

	struct TypeVisitor;

	impl<'ast> Visit<'ast> for TypeVisitor {
		fn visit_type_path(&mut self, i: &'ast TypePath) {
			// println!("visited type {:#?}", i);
			self.visit_path(&i.path);
		}
	}

	let mut visitor = TypeVisitor;
	// visitor.visit_data(&data);

	match *data {
		Data::Struct(ref data) => match &data.fields {
			| Fields::Named(FieldsNamed { named: fields , .. })
			| Fields::Unnamed(FieldsUnnamed { unnamed: fields, .. }) => {
				fields.iter().map(|f| f.ty.clone()).collect()
			},

			Fields::Unit => { vec![] },
		},

		Data::Enum(ref data) => data.variants.iter().flat_map(|variant| {
			match &variant.fields {
				| Fields::Named(FieldsNamed { named: fields , .. })
				| Fields::Unnamed(FieldsUnnamed { unnamed: fields, .. }) => {
					fields.iter().map(|f| f.ty.clone()).collect()
				},

				Fields::Unit => { vec![] },
			}
		})
		.collect(),

		Data::Union(_) => panic!("Union types are not supported."),
	}
}

fn add_trait_bounds(mut generics: Generics, bounds: syn::TypeParamBound) -> Generics {
	for param in &mut generics.params {
		if let GenericParam::Type(ref mut type_param) = *param {
			type_param.bounds.push(bounds.clone());
		}
	}
	generics
}
