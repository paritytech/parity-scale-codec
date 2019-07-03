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
use syn::{Data, Fields, Field, spanned::Spanned, Error};
use crate::utils;
use std::iter::FromIterator;

pub struct Impl {
	pub decode: TokenStream,
	pub min_encoded_len: TokenStream,
}

pub fn quote(data: &Data, type_name: &Ident, input: &TokenStream) -> Result<Impl, TokenStream> {
	let call_site = Span::call_site();
	match *data {
		Data::Struct(ref data) => match data.fields {
			Fields::Named(_) | Fields::Unnamed(_) => fields_impl(
				call_site,
				quote! { #type_name },
				input,
				&data.fields,
			),
			Fields::Unit => {
				let decode = quote_spanned! {call_site =>
					drop(#input);
					Ok(#type_name)
				};

				let min_encoded_len = quote_spanned! {call_site =>
					0
				};

				Ok(Impl { decode, min_encoded_len })
			},
		},
		Data::Enum(ref data) => {
			let data_variants = || data.variants.iter().filter(|variant| crate::utils::get_skip(&variant.attrs).is_none());

			if data_variants().count() > 256 {
				return Err(Error::new(
					Span::call_site(),
					"Currently only enums with at most 256 variants are encodable."
				).to_compile_error());
			}

			let recurse = data_variants().enumerate().map(|(i, v)| {
				let name = &v.ident;
				let index = utils::index(v, i);

				let impl_ = fields_impl(
					call_site,
					quote! { #type_name :: #name },
					input,
					&v.fields,
				)?;

				let impl_decode = impl_.decode;
				let impl_min_encoded_len = impl_.min_encoded_len;

				let decode = quote_spanned! { v.span() =>
					x if x == #index as u8 => {
						#impl_decode
					},
				};

				let min_encoded_len = quote_spanned! { v.span() =>
					1 + #impl_min_encoded_len
				};

				Ok(Impl { decode, min_encoded_len })
			});

			let recurse: Vec<_> = Result::<_, TokenStream>::from_iter(recurse)?;

			let recurse_decode = recurse.iter().map(|i| &i.decode);
			let recurse_min_encoded_len = recurse.iter().map(|i| &i.min_encoded_len);

			let err_msg = format!("No such variant in enum {}", type_name);
			let decode = quote! {
				match #input.read_byte()? {
					#( #recurse_decode )*
					x => Err(#err_msg.into()),
				}
			};

			let min_encoded_len = quote! {
				let mut res = usize::max_value();
				#( res = res.min( #recurse_min_encoded_len); )*
				res
			};

			Ok(Impl { decode, min_encoded_len })
		},
		Data::Union(_) => Err(
			Error::new(Span::call_site(), "Union types are not supported.").to_compile_error()
		),
	}
}

fn field_impl(field: &Field, name: &String, input: &TokenStream) -> Result<Impl, TokenStream> {
	let encoded_as = utils::get_encoded_as_type(field);
	let compact = utils::get_enable_compact(field);
	let skip = utils::get_skip(&field.attrs).is_some();

	if encoded_as.is_some() as u8 + compact as u8 + skip as u8 > 1 {
		return Err(Error::new(
			Span::call_site(),
			"`encoded_as`, `compact` and `skip` can only be used one at a time!"
		).to_compile_error());
	}

	let err_msg = format!("Error decoding field {}", name);


	if compact {
		let field_type = &field.ty;
		let decode = quote_spanned! { field.span() =>
			{
				let res = <
					<#field_type as _parity_scale_codec::HasCompact>::Type as _parity_scale_codec::Decode
				>::decode(#input);
				match res {
					Err(_) => return Err(#err_msg.into()),
					Ok(a) => a.into(),
				}
			}
		};

		let min_encoded_len = quote_spanned! { field.span() =>
			<
				<#field_type as _parity_scale_codec::HasCompact>::Type as _parity_scale_codec::Decode
			>::min_encoded_len()
		};

		Ok(Impl { decode, min_encoded_len })
	} else if let Some(encoded_as) = encoded_as {
		let decode = quote_spanned! { field.span() =>
			{
				let res = <#encoded_as as _parity_scale_codec::Decode>::decode(#input);
				match res {
					Err(_) => return Err(#err_msg.into()),
					Ok(a) => a.into(),
				}
			}
		};

		let min_encoded_len = quote_spanned! { field.span() =>
			<#encoded_as as _parity_scale_codec::Decode>::min_encoded_len()
		};

		Ok(Impl { decode, min_encoded_len })
	} else if skip {
		let decode = quote_spanned! { field.span() => Default::default() };

		let min_encoded_len = quote_spanned! { field.span() => 0 };

		Ok(Impl { decode, min_encoded_len })
	} else {
		let field_type = &field.ty;
		let decode = quote_spanned! { field.span() =>
			{
				let res = _parity_scale_codec::Decode::decode(#input);
				match res {
					Err(_) => return Err(#err_msg.into()),
					Ok(a) => a,
				}
			}
		};

		let min_encoded_len = quote_spanned! { field.span() =>
			<#field_type as _parity_scale_codec::Decode>::min_encoded_len()
		};

		Ok(Impl { decode, min_encoded_len })
	}
}

fn fields_impl(
	call_site: Span,
	name: TokenStream,
	input: &TokenStream,
	fields: &Fields
) -> Result<Impl, TokenStream> {
	match *fields {
		Fields::Named(ref fields) => {
			let recurse = fields.named.iter().map(|f| {
				let name_ident = &f.ident;
				let field = match name_ident {
					Some(a) => format!("{}.{}", name, a),
					None => format!("{}", name),
				};
				let impl_ = field_impl(f, &field, input)?;

				let impl_decode = impl_.decode;

				let decode = quote_spanned! { f.span() =>
					#name_ident: #impl_decode
				};

				let impl_min_encoded_len = impl_.min_encoded_len;
				let min_encoded_len = quote_spanned! { f.span() =>
					#impl_min_encoded_len
				};

				Ok(Impl { decode, min_encoded_len })
			});

			let recurse: Vec<_> = Result::<_, TokenStream>::from_iter(recurse)?;

			let recurse_decode = recurse.iter().map(|i| &i.decode);
			let recurse_min_encoded_len = recurse.iter().map(|i| &i.min_encoded_len);

			let decode = quote_spanned! {call_site =>
				Ok(#name {
					#( #recurse_decode, )*
				})
			};

			let min_encoded_len = quote_spanned! {call_site =>
				0 #( + #recurse_min_encoded_len )*
			};

			Ok(Impl { decode, min_encoded_len })
		},
		Fields::Unnamed(ref fields) => {
			let recurse = fields.unnamed.iter().enumerate().map(|(i, f) | {
				let name = format!("{}.{}", name, i);

				field_impl(f, &name, input)
			});

			let recurse: Vec<_> = Result::from_iter(recurse)?;

			let recurse_decode = recurse.iter().map(|i| &i.decode);
			let recurse_min_encoded_len = recurse.iter().map(|i| &i.min_encoded_len);

			let decode = quote_spanned! {call_site =>
				Ok(#name (
					#( #recurse_decode, )*
				))
			};

			let min_encoded_len = quote_spanned! {call_site =>
				0 #( + #recurse_min_encoded_len )*
			};

			Ok(Impl { decode, min_encoded_len })
		},
		Fields::Unit => {
			let decode = quote_spanned! {call_site =>
				Ok(#name)
			};

			let min_encoded_len = quote_spanned! {call_site =>
				0
			};

			Ok(Impl { decode, min_encoded_len })
		},
	}
}
