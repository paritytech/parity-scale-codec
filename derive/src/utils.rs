// Copyright 2018-2020 Parity Technologies
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

//! Various internal utils.
//!
//! NOTE: attributes finder must be checked using check_attribute first,
//! otherwise the macro can panic.

use std::str::FromStr;

use proc_macro2::TokenStream;
use syn::{
	spanned::Spanned,
	Meta, NestedMeta, Lit, Attribute, Variant, Field, DeriveInput, Fields, Data, FieldsUnnamed,
	FieldsNamed, MetaNameValue, punctuated::Punctuated, token, parse::Parse,
};

fn find_meta_item<'a, F, R, I, M>(mut itr: I, mut pred: F) -> Option<R> where
	F: FnMut(M) -> Option<R> + Clone,
	I: Iterator<Item=&'a Attribute>,
	M: Parse,
{
	itr.find_map(|attr| attr.path.is_ident("codec").then(|| pred(attr.parse_args().ok()?)).flatten())
}

/// Look for a `#[scale(index = $int)]` attribute on a variant. If no attribute
/// is found, fall back to the discriminant or just the variant index.
pub fn variant_index(v: &Variant, i: usize) -> TokenStream {
	// first look for an attribute
	let index = find_meta_item(v.attrs.iter(), |meta| {
		if let NestedMeta::Meta(Meta::NameValue(ref nv)) = meta {
			if nv.path.is_ident("index") {
				if let Lit::Int(ref v) = nv.lit {
					let byte = v.base10_parse::<u8>()
						.expect("Internal error, index attribute must have been checked");
					return Some(byte)
				}
			}
		}

		None
	});

	// then fallback to discriminant or just index
	index.map(|i| quote! { #i })
		.unwrap_or_else(|| v.discriminant
			.as_ref()
			.map(|&(_, ref expr)| quote! { #expr })
			.unwrap_or_else(|| quote! { #i })
		)
}

/// Look for a `#[codec(encoded_as = "SomeType")]` outer attribute on the given
/// `Field`.
pub fn get_encoded_as_type(field: &Field) -> Option<TokenStream> {
	find_meta_item(field.attrs.iter(), |meta| {
		if let NestedMeta::Meta(Meta::NameValue(ref nv)) = meta {
			if nv.path.is_ident("encoded_as") {
				if let Lit::Str(ref s) = nv.lit {
					return Some(
						TokenStream::from_str(&s.value())
							.expect("Internal error, encoded_as attribute must have been checked")
					);
				}
			}
		}

		None
	})
}

/// Look for a `#[codec(compact)]` outer attribute on the given `Field`.
pub fn is_compact(field: &Field) -> bool {
	find_meta_item(field.attrs.iter(), |meta| {
		if let NestedMeta::Meta(Meta::Path(ref path)) = meta {
			if path.is_ident("compact") {
				return Some(());
			}
		}

		None
	}).is_some()
}

/// Look for a `#[codec(skip)]` in the given attributes.
pub fn should_skip(attrs: &[Attribute]) -> bool {
	find_meta_item(attrs.iter(), |meta| {
		if let NestedMeta::Meta(Meta::Path(ref path)) = meta {
			if path.is_ident("skip") {
				return Some(path.span());
			}
		}

		None
	}).is_some()
}

/// Look for a `#[codec(dumb_trait_bound)]`in the given attributes.
pub fn has_dumb_trait_bound(attrs: &[Attribute]) -> bool {
	find_meta_item(attrs.iter(), |meta| {
		if let NestedMeta::Meta(Meta::Path(ref path)) = meta {
			if path.is_ident("dumb_trait_bound") {
				return Some(());
			}
		}

		None
	}).is_some()
}

/// Trait bounds.
pub type TraitBounds = Punctuated<syn::WherePredicate, token::Comma>;

/// Parse `name(T: Bound, N: Bound)` as a custom trait bound.
struct CustomTraitBound<N> {
	_name: N,
	_paren_token: token::Paren,
	bounds: TraitBounds,
}

impl<N: Parse> Parse for CustomTraitBound<N> {
	fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
		let content;
		Ok(Self {
			_name: input.parse()?,
			_paren_token: syn::parenthesized!(content in input),
			bounds: content.parse_terminated(syn::WherePredicate::parse)?,
		})
	}
}

syn::custom_keyword!(encode_bound);
syn::custom_keyword!(decode_bound);

/// Look for a `#[codec(decode_bound(T: Decode))]`in the given attributes.
///
/// If found, it should be used as trait bounds when deriving the `Decode` trait.
pub fn custom_decode_trait_bound(attrs: &[Attribute]) -> Option<TraitBounds> {
	find_meta_item(attrs.iter(), |meta: CustomTraitBound<decode_bound>| {
		Some(meta.bounds)
	})
}

/// Look for a `#[codec(encode_bound(T: Encode))]`in the given attributes.
///
/// If found, it should be used as trait bounds when deriving the `Encode` trait.
pub fn custom_encode_trait_bound(attrs: &[Attribute]) -> Option<TraitBounds> {
	find_meta_item(attrs.iter(), |meta: CustomTraitBound<encode_bound>| {
		Some(meta.bounds)
	})
}

/// Given a set of named fields, return an iterator of `Field` where all fields
/// marked `#[codec(skip)]` are filtered out.
pub fn filter_skip_named<'a>(fields: &'a syn::FieldsNamed) -> impl Iterator<Item=&Field> + 'a {
	fields.named.iter()
		.filter(|f| !should_skip(&f.attrs))
}

/// Given a set of unnamed fields, return an iterator of `(index, Field)` where all fields
/// marked `#[codec(skip)]` are filtered out.
pub fn filter_skip_unnamed<'a>(fields: &'a syn::FieldsUnnamed) -> impl Iterator<Item=(usize, &Field)> + 'a {
	fields.unnamed.iter()
		.enumerate()
		.filter(|(_, f)| !should_skip(&f.attrs))
}

/// Ensure attributes are correctly applied. This *must* be called before using
/// any of the attribute finder methods or the macro may panic if it encounters
/// misapplied attributes.
/// `#[codec(dumb_trait_bound)]` is the only accepted top attribute.
/// Fields can have the following attributes:
/// * `#[codec(skip)]`
/// * `#[codec(compact)]`
/// * `#[codec(encoded_as = "$EncodeAs")]` with $EncodedAs a valid TokenStream
/// Variants can have the following attributes:
/// * `#[codec(skip)]`
/// * `#[codec(index = $int)]`
pub fn check_attributes(input: &DeriveInput) -> syn::Result<()> {
	for attr in &input.attrs {
		check_top_attribute(attr)?;
	}

	match input.data {
		Data::Struct(ref data) => match &data.fields {
			| Fields::Named(FieldsNamed { named: fields , .. })
			| Fields::Unnamed(FieldsUnnamed { unnamed: fields, .. }) => {
				for field in fields {
					for attr in &field.attrs {
						check_field_attribute(attr)?;
					}
				}
			}
			Fields::Unit => (),
		}
		Data::Enum(ref data) => {
			for variant in data.variants.iter() {
				for attr in &variant.attrs {
					check_variant_attribute(attr)?;
				}
				for field in &variant.fields {
					for attr in &field.attrs {
						check_field_attribute(attr)?;
					}
				}
			}
		},
		Data::Union(_) => (),
	}
	Ok(())
}

// Ensure a field is decorated only with the following attributes:
// * `#[codec(skip)]`
// * `#[codec(compact)]`
// * `#[codec(encoded_as = "$EncodeAs")]` with $EncodedAs a valid TokenStream
fn check_field_attribute(attr: &Attribute) -> syn::Result<()> {
	let field_error = "Invalid attribute on field, only `#[codec(skip)]`, `#[codec(compact)]` and \
		`#[codec(encoded_as = \"$EncodeAs\")]` are accepted.";

	if attr.path.is_ident("codec") {
		match attr.parse_meta()? {
			Meta::List(ref meta_list) if meta_list.nested.len() == 1 => {
				match meta_list.nested.first().expect("Just checked that there is one item; qed") {
					NestedMeta::Meta(Meta::Path(path))
						if path.get_ident().map_or(false, |i| i == "skip") => Ok(()),

					NestedMeta::Meta(Meta::Path(path))
						if path.get_ident().map_or(false, |i| i == "compact") => Ok(()),

					NestedMeta::Meta(Meta::NameValue(MetaNameValue { path, lit: Lit::Str(lit_str), .. }))
						if path.get_ident().map_or(false, |i| i == "encoded_as")
					=> TokenStream::from_str(&lit_str.value()).map(|_| ())
						.map_err(|_e| syn::Error::new(lit_str.span(), "Invalid token stream")),

					elt @ _ => Err(syn::Error::new(elt.span(), field_error)),
				}
			},
			meta @ _ => Err(syn::Error::new(meta.span(), field_error)),
		}
	} else {
		Ok(())
	}
}

// Ensure a field is decorated only with the following attributes:
// * `#[codec(skip)]`
// * `#[codec(index = $int)]`
fn check_variant_attribute(attr: &Attribute) -> syn::Result<()> {
	let variant_error = "Invalid attribute on variant, only `#[codec(skip)]` and \
		`#[codec(index = $u8)]` are accepted.";

	if attr.path.is_ident("codec") {
		match attr.parse_meta()? {
			Meta::List(ref meta_list) if meta_list.nested.len() == 1 => {
				match meta_list.nested.first().expect("Just checked that there is one item; qed") {
					NestedMeta::Meta(Meta::Path(path))
						if path.get_ident().map_or(false, |i| i == "skip") => Ok(()),

					NestedMeta::Meta(Meta::NameValue(MetaNameValue { path, lit: Lit::Int(lit_int), .. }))
						if path.get_ident().map_or(false, |i| i == "index")
					=> lit_int.base10_parse::<u8>().map(|_| ())
						.map_err(|_| syn::Error::new(lit_int.span(), "Index must be in 0..255")),

					elt @ _ => Err(syn::Error::new(elt.span(), variant_error)),
				}
			},
			meta @ _ => Err(syn::Error::new(meta.span(), variant_error)),
		}
	} else {
		Ok(())
	}
}

// Only `#[codec(dumb_trait_bound)]` is accepted as top attribute
fn check_top_attribute(attr: &Attribute) -> syn::Result<()> {
	let top_error =
		"Invalid attribute only `#[codec(dumb_trait_bound)]`, `#[codec(encode_bound(T: Encode))]` or \
		`#[codec(decode_bound(T: Decode))]` are accepted as top attribute";
	if attr.path.is_ident("codec") {
		if attr.parse_args::<CustomTraitBound<encode_bound>>().is_ok() {
			return Ok(())
		} else if attr.parse_args::<CustomTraitBound<decode_bound>>().is_ok() {
			return Ok(())
		} else {
			match attr.parse_meta()? {
				Meta::List(ref meta_list) if meta_list.nested.len() == 1 => {
					match meta_list.nested.first().expect("Just checked that there is one item; qed") {
						NestedMeta::Meta(Meta::Path(path))
							if path.get_ident().map_or(false, |i| i == "dumb_trait_bound") => Ok(()),

						elt @ _ => Err(syn::Error::new(elt.span(), top_error)),
					}
				},
				_ => Err(syn::Error::new(attr.span(), top_error)),
			}
		}
	} else {
		Ok(())
	}
}
