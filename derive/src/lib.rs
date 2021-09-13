// Copyright 2017-2021 Parity Technologies
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

#[macro_use]
extern crate syn;

#[macro_use]
extern crate quote;

use proc_macro2::{Ident, Span};
use proc_macro_crate::{crate_name, FoundCrate};
use syn::spanned::Spanned;
use syn::{Data, Field, Fields, DeriveInput, Error};
use proc_macro2::TokenStream as TokenStream2;
use crate::utils::is_lint_attribute;

mod decode;
mod encode;
mod max_encoded_len;
mod utils;
mod trait_bounds;

/// Returns the identifier of the `parity-scale-codec` crate as used.
///
/// The identifier might change if the depending crate imported it
/// using a custom package name.
fn parity_scale_codec_ident() -> Result<TokenStream2, Error> {
	static CRATE_NAME: &str = "parity-scale-codec";
	fn root_import(name: &str) -> TokenStream2 {
		let ident = Ident::new(name, Span::call_site());
		quote!{ :: #ident }
	}
	// This "hack" is required for the tests.
	if std::env::var("CARGO_PKG_NAME").unwrap() == CRATE_NAME {
		Ok(root_import("parity_scale_codec"))
	} else {
		match crate_name(CRATE_NAME) {
			Ok(FoundCrate::Itself) => {
				Ok(quote! { crate })
			}
			Ok(FoundCrate::Name(custom_name)) => Ok(root_import(&custom_name)),
			Err(e) => Err(Error::new(Span::call_site(), &e)),
		}
	}
}

/// Wraps the impl block in a "dummy const"
fn wrap_with_dummy_const(input: DeriveInput, impl_block: proc_macro2::TokenStream) -> proc_macro::TokenStream {
	let attrs = input.attrs.into_iter().filter(is_lint_attribute);
	let generated = quote! {
		const _: () = {
			#(#attrs)*
			#impl_block
		};
	};

	generated.into()
}

/// Derive `parity_scale_codec::Encode` and `parity_scale_codec::EncodeLike` for struct and enum.
///
/// # Struct
///
/// A struct is encoded by encoding each of its fields successively.
///
/// Fields can have some attributes:
/// * `#[codec(skip)]`: the field is not encoded. It must derive `Default` if Decode is derived.
/// * `#[codec(compact)]`: the field is encoded in its compact representation i.e. the field must
///   implement `parity_scale_codec::HasCompact` and will be encoded as `HasCompact::Type`.
/// * `#[codec(encoded_as = "$EncodeAs")]`: the field is encoded as an alternative type. $EncodedAs
///   type must implement `parity_scale_codec::EncodeAsRef<'_, $FieldType>` with $FieldType the
///   type of the field with the attribute. This is intended to be used for types implementing
///   `HasCompact` as shown in the example.
/// * `#[codec(encode_bound(T: Encode))]`: a custom where bound that will be used when deriving the `Encode` trait.
/// * `#[codec(decode_bound(T: Encode))]`: a custom where bound that will be used when deriving the `Decode` trait.
///
/// ```
/// # use parity_scale_codec_derive::Encode;
/// # use parity_scale_codec::{Encode as _, HasCompact};
/// #[derive(Encode)]
/// struct StructType {
///		#[codec(skip)]
///		a: u32,
///		#[codec(compact)]
///		b: u32,
///		#[codec(encoded_as = "<u32 as HasCompact>::Type")]
///		c: u32,
/// }
/// ```
///
/// # Enum
///
/// The variable is encoded with one byte for the variant and then the variant struct encoding.
/// The variant number is:
/// * if variant has attribute: `#[codec(index = "$n")]` then n
/// * else if variant has discrimant (like 3 in `enum T { A = 3 }`) then the discrimant.
/// * else its position in the variant set, excluding skipped variants, but including variant with
/// discrimant or attribute. Warning this position does collision with discrimant or attribute
/// index.
///
/// variant attributes:
/// * `#[codec(skip)]`: the variant is not encoded.
/// * `#[codec(index = "$n")]`: override variant index.
///
/// field attributes: same as struct fields attributes.
///
/// ```
/// # use parity_scale_codec_derive::Encode;
/// # use parity_scale_codec::Encode as _;
/// #[derive(Encode)]
/// enum EnumType {
/// 	#[codec(index = 15)]
/// 	A,
/// 	#[codec(skip)]
/// 	B,
/// 	C = 3,
/// 	D,
/// }
///
/// assert_eq!(EnumType::A.encode(), vec![15]);
/// assert_eq!(EnumType::B.encode(), vec![]);
/// assert_eq!(EnumType::C.encode(), vec![3]);
/// assert_eq!(EnumType::D.encode(), vec![2]);
/// ```
#[proc_macro_derive(Encode, attributes(codec))]
pub fn encode_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let mut input: DeriveInput = match syn::parse(input) {
		Ok(input) => input,
		Err(e) => return e.to_compile_error().into(),
	};

	if let Err(e) = utils::check_attributes(&input) {
		return e.to_compile_error().into();
	}

	let crate_ident = match crate::parity_scale_codec_ident() {
		Ok(crate_ident) => crate_ident,
		Err(error) => {
			return error.into_compile_error().into()
		}
	};

	if let Some(custom_bound) = utils::custom_encode_trait_bound(&input.attrs) {
		input.generics.make_where_clause().predicates.extend(custom_bound);
	} else if let Err(e) = trait_bounds::add(
		&input.ident,
		&mut input.generics,
		&input.data,
		parse_quote!(#crate_ident::Encode),
		None,
		utils::has_dumb_trait_bound(&input.attrs),
		&crate_ident,
	) {
		return e.to_compile_error().into();
	}

	let name = &input.ident;
	let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

	let encode_impl = encode::quote(&input.data, name, &crate_ident);

	let impl_block = quote! {
		impl #impl_generics #crate_ident::Encode for #name #ty_generics #where_clause {
			#encode_impl
		}

		impl #impl_generics #crate_ident::EncodeLike for #name #ty_generics #where_clause {}
	};

	wrap_with_dummy_const(input, impl_block)
}

/// Derive `parity_scale_codec::Decode` and for struct and enum.
///
/// see derive `Encode` documentation.
#[proc_macro_derive(Decode, attributes(codec))]
pub fn decode_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let mut input: DeriveInput = match syn::parse(input) {
		Ok(input) => input,
		Err(e) => return e.to_compile_error().into(),
	};

	if let Err(e) = utils::check_attributes(&input) {
		return e.to_compile_error().into();
	}

	let crate_ident = match crate::parity_scale_codec_ident() {
		Ok(crate_ident) => crate_ident,
		Err(error) => {
			return error.into_compile_error().into()
		}
	};

	if let Some(custom_bound) = utils::custom_decode_trait_bound(&input.attrs) {
		input.generics.make_where_clause().predicates.extend(custom_bound);
	} else if let Err(e) = trait_bounds::add(
		&input.ident,
		&mut input.generics,
		&input.data,
		parse_quote!(#crate_ident::Decode),
		Some(parse_quote!(Default)),
		utils::has_dumb_trait_bound(&input.attrs),
		&crate_ident,
	) {
		return e.to_compile_error().into();
	}

	let name = &input.ident;
	let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
	let ty_gen_turbofish = ty_generics.as_turbofish();

	let input_ = quote!(__codec_input_edqy);
	let decoding = decode::quote(&input.data, name, &quote!(#ty_gen_turbofish), &input_, &crate_ident);

	let impl_block = quote! {
		impl #impl_generics #crate_ident::Decode for #name #ty_generics #where_clause {
			fn decode<__CodecInputEdqy: #crate_ident::Input>(
				#input_: &mut __CodecInputEdqy
			) -> ::core::result::Result<Self, #crate_ident::Error> {
				#decoding
			}
		}
	};

	wrap_with_dummy_const(input, impl_block)
}

/// Derive `parity_scale_codec::Compact` and `parity_scale_codec::CompactAs` for struct with single
/// field.
///
/// Attribute skip can be used to skip other fields.
///
/// # Example
///
/// ```
/// # use parity_scale_codec_derive::CompactAs;
/// # use parity_scale_codec::{Encode, HasCompact};
/// # use std::marker::PhantomData;
/// #[derive(CompactAs)]
/// struct MyWrapper<T>(u32, #[codec(skip)] PhantomData<T>);
/// ```
#[proc_macro_derive(CompactAs, attributes(codec))]
pub fn compact_as_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let mut input: DeriveInput = match syn::parse(input) {
		Ok(input) => input,
		Err(e) => return e.to_compile_error().into(),
	};

	if let Err(e) = utils::check_attributes(&input) {
		return e.to_compile_error().into();
	}

	let crate_ident = match crate::parity_scale_codec_ident() {
		Ok(crate_ident) => crate_ident,
		Err(error) => {
			return error.into_compile_error().into()
		}
	};

	if let Err(e) = trait_bounds::add(
		&input.ident,
		&mut input.generics,
		&input.data,
		parse_quote!(#crate_ident::CompactAs),
		None,
		utils::has_dumb_trait_bound(&input.attrs),
		&crate_ident,
	) {
		return e.to_compile_error().into();
	}

	let name = &input.ident;
	let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

	fn val_or_default(field: &Field) -> proc_macro2::TokenStream {
		let skip = utils::should_skip(&field.attrs);
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
		impl #impl_generics #crate_ident::CompactAs for #name #ty_generics #where_clause {
			type As = #inner_ty;
			fn encode_as(&self) -> &#inner_ty {
				#inner_field
			}
			fn decode_from(x: #inner_ty)
				-> ::core::result::Result<#name #ty_generics, #crate_ident::Error>
			{
				::core::result::Result::Ok(#constructor)
			}
		}

		impl #impl_generics From<#crate_ident::Compact<#name #ty_generics>>
			for #name #ty_generics #where_clause
		{
			fn from(x: #crate_ident::Compact<#name #ty_generics>) -> #name #ty_generics {
				x.0
			}
		}
	};

	wrap_with_dummy_const(input, impl_block)
}

/// Derive `MaxEncodedLen`.
#[cfg(feature = "max-encoded-len")]
#[proc_macro_derive(MaxEncodedLen, attributes(max_encoded_len_mod))]
pub fn derive_max_encoded_len(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	max_encoded_len::derive_max_encoded_len(input)
}
