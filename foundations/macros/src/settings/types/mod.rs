use syn::{punctuated::Punctuated, Meta};

use self::{enum_ty::Enum, struct_ty::Struct};

mod enum_ty;
mod field_ty;
mod serde;
mod struct_ty;
mod variant_ty;

#[derive(Debug, Clone)]
pub enum EnumOrStruct {
    Enum(Enum),
    Struct(Struct),
}

impl EnumOrStruct {
    pub fn new(item: syn::Item) -> syn::Result<Self> {
        match item {
            syn::Item::Enum(item) => Ok(Self::Enum(Enum::new(item)?)),
            syn::Item::Struct(item) => Ok(Self::Struct(Struct::new(item)?)),
            item => Err(syn::Error::new_spanned(item, "expected enum or struct")),
        }
    }

    pub fn args(&self) -> &GlobalArgs {
        match self {
            Self::Enum(item) => &item.args.global,
            Self::Struct(item) => &item.args.global,
        }
    }

    pub fn default_impl(&self) -> syn::Result<proc_macro2::TokenStream> {
        match self {
            Self::Enum(item) => item.default_impl(),
            Self::Struct(item) => item.default_impl(),
        }
    }

    pub fn docs_impl(&self, crate_path: &syn::Path) -> syn::Result<proc_macro2::TokenStream> {
        match self {
            Self::Enum(item) => item.docs_impl(crate_path),
            Self::Struct(item) => item.docs_impl(crate_path),
        }
    }
}

trait Args: Default {
    fn parse(attrs: &[syn::Attribute]) -> syn::Result<Self>
    where
        Self: Sized,
    {
        attrs
            .into_iter()
            .filter(|a| a.path().is_ident("settings"))
            .try_fold(Self::default(), |mut state, attr| {
                let Meta::List(meta) = &attr.meta else {
                    return Err(syn::Error::new_spanned(&attr, "expected #[settings(...)]"));
                };

                let parsed =
                    meta.parse_args_with(Punctuated::<Meta, syn::Token![,]>::parse_terminated)?;

                for meta in parsed {
                    if !state.apply_meta(&meta)? {
                        return Err(syn::Error::new_spanned(meta, "unexpected setting"));
                    }
                }

                Ok(state)
            })
    }

    fn apply_meta(&mut self, meta: &Meta) -> syn::Result<bool>;
}

#[derive(Debug, Clone)]
pub struct GlobalArgs {
    pub impl_default: bool,
    pub crate_path: syn::Path,
}

impl Default for GlobalArgs {
    fn default() -> Self {
        Self {
            impl_default: true,
            crate_path: syn::parse(quote::quote! { scuffle_foundations }.into()).unwrap(),
        }
    }
}

impl Args for GlobalArgs {
    fn apply_meta(&mut self, meta: &Meta) -> syn::Result<bool> {
        match meta {
            Meta::NameValue(meta) if meta.path.is_ident("impl_default") => {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Bool(lit),
                    ..
                }) = &meta.value
                {
                    self.impl_default = lit.value();
                    Ok(true)
                } else {
                    Err(syn::Error::new_spanned(&meta.value, "expected boolean"))
                }
            }
            Meta::NameValue(meta) if meta.path.is_ident("crate_path") => {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(lit),
                    ..
                }) = &meta.value
                {
                    self.crate_path = syn::parse_str(&lit.value())
                        .map_err(|_| syn::Error::new_spanned(&lit, "expected valid path"))?;
                    Ok(true)
                } else {
                    Err(syn::Error::new_spanned(&meta.value, "expected string"))
                }
            }
            _ => Ok(false),
        }
    }
}
