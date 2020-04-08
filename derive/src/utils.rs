// Copyright 2018 Parity Technologies
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
//! NOTE: attributes finder must be checked using check_attribute first, otherwise macro can panic.

use std::str::FromStr;

use proc_macro2::{TokenStream, Span};
use syn::{
	spanned::Spanned,
	Meta, NestedMeta, Lit, Attribute, Variant, Field, DeriveInput, Fields, Data, FieldsUnnamed,
	FieldsNamed, MetaNameValue
};

fn find_meta_item<'a, F, R, I>(itr: I, pred: F) -> Option<R> where
	F: FnMut(&NestedMeta) -> Option<R> + Clone,
	I: Iterator<Item=&'a Attribute>
{
	itr.filter_map(|attr| {
		if attr.path.is_ident("codec") {
			if let Meta::List(ref meta_list) = attr.parse_meta()
				.expect("Internal error, parse_meta must have been checked")
			{
				return meta_list.nested.iter().filter_map(pred.clone()).next();
			}
		}

		None
	}).next()
}

pub fn index(v: &Variant, i: usize) -> TokenStream {
	// look for an index in attributes
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

pub fn get_encoded_as_type(field_entry: &Field) -> Option<TokenStream> {
	// look for an encoded_as in attributes
	find_meta_item(field_entry.attrs.iter(), |meta| {
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

pub fn get_enable_compact(field_entry: &Field) -> bool {
	// look for `encode(compact)` in the attributes
	find_meta_item(field_entry.attrs.iter(), |meta| {
		if let NestedMeta::Meta(Meta::Path(ref path)) = meta {
			if path.is_ident("compact") {
				return Some(());
			}
		}

		None
	}).is_some()
}

// return span of skip if found
pub fn get_skip(attrs: &[Attribute]) -> Option<Span> {
	// look for `skip` in the attributes
	find_meta_item(attrs.iter(), |meta| {
		if let NestedMeta::Meta(Meta::Path(ref path)) = meta {
			if path.is_ident("skip") {
				return Some(path.span());
			}
		}

		None
	})
}

/// Returns if the `dumb_trait_bound` attribute is given in `attrs`.
pub fn get_dumb_trait_bound(attrs: &[Attribute]) -> bool {
	find_meta_item(attrs.iter(), |meta| {
		if let NestedMeta::Meta(Meta::Path(ref path)) = meta {
			if path.is_ident("dumb_trait_bound") {
				return Some(());
			}
		}

		None
	}).is_some()
}

pub fn filter_skip_named<'a>(fields: &'a syn::FieldsNamed) -> impl Iterator<Item=&Field> + 'a {
	fields.named.iter()
		.filter(|f| get_skip(&f.attrs).is_none())
}

pub fn filter_skip_unnamed<'a>(fields: &'a syn::FieldsUnnamed) -> impl Iterator<Item=(usize, &Field)> + 'a {
	fields.unnamed.iter()
		.enumerate()
		.filter(|(_, f)| get_skip(&f.attrs).is_none())
}

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

// Is accepted only:
// * `#[codec(skip)]`
// * `#[codec(compact)]`
// * `#[codec(encoded_as = "$EncodeAs")]` with $EncodedAs a valid TokenStream
fn check_field_attribute(attr: &Attribute) -> syn::Result<()> {
	let field_error = "Invalid attribute on field, only `#[codec(skip)]`, `#[codec(compact)]` and \
		`#[codec(encoded_as = \"$EncodeAs\")]` are accepted.";

	if attr.path.is_ident("codec") {
		match attr.parse_meta()? {
			Meta::List(ref meta_list) if meta_list.nested.len() == 1 => {
				match meta_list.nested.first().unwrap() {
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

// Is accepted only:
// * `#[codec(skip)]`
// * `#[codec(index = $int)]`
fn check_variant_attribute(attr: &Attribute) -> syn::Result<()> {
	let variant_error = "Invalid attribute on variant, only `#[codec(skip)]` and \
		`#[codec(index = $u8)]` are accepted.";

	if attr.path.is_ident("codec") {
		match attr.parse_meta()? {
			Meta::List(ref meta_list) if meta_list.nested.len() == 1 => {
				match meta_list.nested.first().unwrap() {
					NestedMeta::Meta(Meta::Path(path))
						if path.get_ident().map_or(false, |i| i == "skip") => Ok(()),

					NestedMeta::Meta(Meta::NameValue(MetaNameValue { path, lit: Lit::Int(lit_int), .. }))
						if path.get_ident().map_or(false, |i| i == "index")
					=> lit_int.base10_parse::<u8>().map(|_| ()),

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
	let top_error = "Invalid attribute only `#[codec(dumb_trait_bound)]` is accepted as top \
		attribute";
	if attr.path.is_ident("codec") {
		match attr.parse_meta()? {
			Meta::List(ref meta_list) if meta_list.nested.len() == 1 => {
				match meta_list.nested.first().unwrap() {
					NestedMeta::Meta(Meta::Path(path))
						if path.get_ident().map_or(false, |i| i == "dumb_trait_bound") => Ok(()),

					elt @ _ => Err(syn::Error::new(elt.span(), top_error)),
				}
			},
			meta @ _ => Err(syn::Error::new(meta.span(), top_error)),
		}
	} else {
		Ok(())
	}
}
