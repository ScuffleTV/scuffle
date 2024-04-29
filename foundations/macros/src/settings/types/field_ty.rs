use syn::{LitStr, Meta};

use crate::helpers::parse_docs;

use super::{
    serde::{parse_default_fn, serde_flatten, Name, RenameAll},
    Args,
};

#[derive(Debug, Clone)]
pub struct Field {
    pub name: Option<Name>,
    pub docs: Vec<LitStr>,
    pub default_fn: Option<proc_macro2::TokenStream>,
    pub args: FieldArgs,
    pub item: syn::Field,
    pub flatten: bool,
}

impl Field {
    pub fn new(item: syn::Field, rename: Option<RenameAll>) -> syn::Result<Self> {
        Ok(Self {
            name: item
                .ident
                .as_ref()
                .map(|ident| Name::parse(&item.attrs, ident.to_string().as_str(), rename))
                .transpose()?,
            docs: parse_docs(&item.attrs),
            default_fn: parse_default_fn(&item.attrs)?,
            args: FieldArgs::parse(&item.attrs)?,
            flatten: serde_flatten(&item.attrs)?,
            item,
        })
    }

    pub fn default_impl(&self) -> proc_macro2::TokenStream {
        let default = self
            .args
            .default
            .as_ref()
            .map(|default| {
                quote::quote! { #default }
            })
            .unwrap_or_else(|| {
                let func = self
                    .default_fn
                    .as_ref()
                    .map(|default_fn| quote::quote! { #default_fn })
                    .unwrap_or_else(|| quote::quote! { Default::default });

                quote::quote! { #func() }
            });

        quote::quote! {
            #default
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct FieldArgs {
    default: Option<syn::Expr>,
}

impl Args for FieldArgs {
    fn apply_meta(&mut self, meta: &Meta) -> syn::Result<bool> {
        match meta {
            Meta::NameValue(meta) if meta.path.is_ident("default") => {
                self.default = Some(meta.value.clone());
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}
