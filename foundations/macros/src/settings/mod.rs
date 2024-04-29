use darling::{ast::NestedMeta, FromMeta};
use proc_macro::TokenStream;
use syn::{parse::Parse, punctuated::Punctuated, Token};

use self::types::EnumOrStruct;

mod types;

#[derive(darling::FromMeta)]
struct Options {
    #[darling(default = "Options::default_crate_path")]
    crate_path: syn::Path,
    #[darling(default = "Options::default_impl_default")]
    impl_default: bool,
    #[darling(default = "Options::default_impl_debug")]
    impl_debug: bool,
    #[darling(default = "Options::default_impl_clone")]
    impl_clone: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            crate_path: Options::default_crate_path(),
            impl_default: Options::default_impl_default(),
            impl_debug: Options::default_impl_debug(),
            impl_clone: Options::default_impl_clone(),
        }
    }
}

impl Options {
    fn default_crate_path() -> syn::Path {
        syn::parse_quote!(scuffle_foundations)
    }

    fn default_impl_default() -> bool {
        true
    }

    fn default_impl_debug() -> bool {
        true
    }

    fn default_impl_clone() -> bool {
        true
    }
}

impl Parse for Options {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.is_empty() {
            Ok(Options::default())
        } else {
            let meta_list = Punctuated::<NestedMeta, Token![,]>::parse_terminated(input)?
                .into_iter()
                .collect::<Vec<_>>();
            Ok(Options::from_list(&meta_list)?)
        }
    }
}

pub fn settings(args: TokenStream, input: TokenStream) -> syn::Result<proc_macro2::TokenStream> {
    let args = syn::parse::<Options>(args)?;
    let input = syn::parse::<syn::Item>(input)?;

    let mut derives = Vec::new();

    if args.impl_clone {
        derives.push(quote::quote!(Clone));
    }

    if args.impl_debug {
        derives.push(quote::quote!(Debug));
    }

    let crate_path = args.crate_path;

    derives.push(quote::quote!(#crate_path::macro_reexports::serde::Deserialize));
    derives.push(quote::quote!(#crate_path::macro_reexports::serde::Serialize));
    derives.push(quote::quote!(#crate_path::settings::Settings));

    let attr = {
        let impl_default = args.impl_default;
        let crate_path = quote::quote!(#crate_path).to_string();
        quote::quote! {
            #[settings(impl_default = #impl_default, crate_path = #crate_path)]
        }
    };

    let serde_path = format!(
        "{}",
        quote::quote! {
            #crate_path::macro_reexports::serde
        }
    );

    Ok(quote::quote! {
        #[derive(#(#derives),*)]
        #[serde(crate = #serde_path)]
        #attr
        #input
    })
}

pub fn derive_settings(input: TokenStream) -> syn::Result<proc_macro2::TokenStream> {
    let input = syn::parse::<syn::Item>(input)?;

    let item = EnumOrStruct::new(input)?;

    let args = item.args();

    let docs = item.docs_impl(&args.crate_path)?;
    let default = if args.impl_default {
        item.default_impl()?
    } else {
        quote::quote! {}
    };

    Ok(quote::quote! {
        #docs
        #default
    })
}
