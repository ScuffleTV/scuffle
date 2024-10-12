use syn::spanned::Spanned;
use syn::{LitStr, Meta};

use super::field_ty::Field;
use super::serde::RenameAll;
use super::Args;
use crate::helpers::parse_docs;

#[derive(Debug, Clone)]
pub struct Variant {
	// pub name: Name,
	pub fields: Vec<Field>,
	pub docs: Vec<LitStr>,
	pub args: VariantArgs,
	pub item: syn::Variant,
}

impl Variant {
	pub fn new(item: syn::Variant, _rename: Option<RenameAll>) -> syn::Result<Self> {
		let field_rename = RenameAll::parse(&item.attrs)?;

		Ok(Self {
			// name: Name::parse(&item.attrs, item.ident.to_string().as_str(), rename)?,
			fields: match &item.fields {
				syn::Fields::Named(fields) => fields
					.named
					.iter()
					.cloned()
					.map(|field| Field::new(field, field_rename))
					.collect::<syn::Result<_>>()?,
				syn::Fields::Unnamed(fields) => fields
					.unnamed
					.iter()
					.cloned()
					.map(|field| Field::new(field, field_rename))
					.collect::<syn::Result<_>>()?,
				syn::Fields::Unit => Vec::new(),
			},
			docs: parse_docs(&item.attrs),
			args: VariantArgs::parse(&item.attrs)?,
			item,
		})
	}

	pub fn default_impl(&self) -> syn::Result<proc_macro2::TokenStream> {
		let ident = &self.item.ident;
		let fields = self.fields.iter().map(|field| {
			let default = field.default_impl();

			match field.item.ident.as_ref() {
				// named field
				Some(ident) => {
					quote::quote! {
						#ident: #default,
					}
				}
				// tuple struct
				None => {
					quote::quote! { #default }
				}
			}
		});

		match &self.item.fields {
			syn::Fields::Named(_) => Ok(quote::quote! {
				Self::#ident {
					#(#fields)*
				}
			}),
			syn::Fields::Unnamed(_) => Ok(quote::quote! {
				Self::#ident(
					#(#fields,)*
				)
			}),
			syn::Fields::Unit => Ok(quote::quote! {
				Self::#ident
			}),
		}
	}

	pub fn docs_impl(
		&self,
		crate_path: &syn::Path,
		kind_tag: Option<&str>,
		content_tag: Option<&str>,
	) -> syn::Result<(proc_macro2::TokenStream, proc_macro2::TokenStream)> {
		let fields = self.fields.iter().enumerate().map(|(idx, field)| {
			let docs = field
				.docs
				.iter()
				.map(|doc| {
					quote::quote! { ::std::borrow::Cow::Borrowed(#doc) }
				})
				.collect::<Vec<_>>();

			let insert = if docs.is_empty() || field.flatten {
				quote::quote! {}
			} else {
				quote::quote! {
					docs.insert(parent_key, ::std::borrow::Cow::Borrowed(&[#(#docs),*]));
				}
			};

			let name_push = if (self.fields.len() == 1 && field.name.is_none()) || field.flatten {
				quote::quote! {}
			} else {
				let name = field
					.name
					.as_ref()
					.map(|name| name.serialize.clone())
					.unwrap_or_else(|| format!("{idx}"));

				quote::quote! {
					parent_key.push(::std::borrow::Cow::Borrowed(#name));
				}
			};

			let name_push = if let Some(content_tag) = content_tag {
				quote::quote! {
					parent_key.push(::std::borrow::Cow::Borrowed(#content_tag));
					#name_push
				}
			} else {
				name_push
			};

			let ident = syn::Ident::new(&format!("__field{}", idx), self.item.span());

			quote::quote! {
				{
					let mut parent_key = parent_key.to_vec();
					#name_push
					(&&&&#crate_path::settings::Wrapped(#ident)).add_docs(&parent_key, docs);
					#insert
				}
			}
		});

		let variant_doc = if !self.docs.is_empty() {
			let docs = self
				.docs
				.iter()
				.map(|doc| {
					quote::quote! { ::std::borrow::Cow::Borrowed(#doc) }
				})
				.collect::<Vec<_>>();

			let kind_tag = kind_tag.unwrap_or(">");

			quote::quote! {
				{
					let mut parent_key = parent_key.to_vec();
					parent_key.push(::std::borrow::Cow::Borrowed(#kind_tag));
					docs.insert(parent_key, ::std::borrow::Cow::Borrowed(&[#(#docs),*]));
				}
			}
		} else {
			quote::quote! {}
		};

		let match_fields = self.fields.iter().enumerate().map(|(idx, field)| {
			let name = syn::Ident::new(&format!("__field{}", idx), self.item.span());
			match &self.item.fields {
				syn::Fields::Named(_) => {
					let ident = field.item.ident.as_ref().unwrap();
					quote::quote! {
						#ident: #name
					}
				}
				syn::Fields::Unnamed(_) => {
					quote::quote! {
						#name
					}
				}
				syn::Fields::Unit => {
					quote::quote! {}
				}
			}
		});

		let ident = &self.item.ident;
		let match_token = match &self.item.fields {
			syn::Fields::Named(_) => {
				quote::quote! { #ident{ #(#match_fields),* } }
			}
			syn::Fields::Unnamed(_) => {
				quote::quote! { #ident( #(#match_fields),* ) }
			}
			syn::Fields::Unit => {
				quote::quote! { #ident }
			}
		};

		let fields = quote::quote! {
			#variant_doc
			#(#fields)*
		};

		Ok((match_token, fields))
	}
}

#[derive(Debug, Clone, Default)]
pub struct VariantArgs {
	pub default: bool,
}

impl Args for VariantArgs {
	fn apply_meta(&mut self, meta: &Meta) -> syn::Result<bool> {
		match meta {
			Meta::Path(path) if path.is_ident("default") => {
				if self.default {
					return Err(syn::Error::new_spanned(path, "duplicate setting"));
				}

				self.default = true;

				Ok(true)
			}
			_ => Ok(false),
		}
	}
}
