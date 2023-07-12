#![doc = include_str!("../README.md")]

use proc_macro::TokenStream;
use quote::quote;
use syn::{DataStruct, Type};

#[proc_macro_derive(Config, attributes(config))]
pub fn derive_answer_fn(tokens: TokenStream) -> TokenStream {
    let ast = syn::parse(tokens).unwrap();
    match impl_config(&ast) {
        Ok(tokens) => tokens,
        Err(e) => e.to_compile_error().into(),
    }
}

fn impl_config(ast: &syn::DeriveInput) -> syn::Result<TokenStream> {
    let syn::Data::Struct(DataStruct {
        fields: syn::Fields::Named(fields),
        ..
    }) = &ast.data
    else {
        return Err(syn::Error::new_spanned(ast, "Only structs are supported"));
    };

    let attributes = get_attributes(&ast.attrs)?;

    let struct_env_attr =
        attributes
            .iter()
            .find_map(|a| if let Attr::Env(e) = a { Some(e) } else { None });

    let struct_cli_attr =
        attributes
            .iter()
            .find_map(|a| if let Attr::Cli(e) = a { Some(e) } else { None });

    let mut keys_init = vec![];

    for field in fields.named.iter() {
        let Some(ident) = &field.ident else {
            return Err(syn::Error::new_spanned(
                field,
                "Only named fields are supported",
            ));
        };

        let attributes = get_attributes(&field.attrs)?;

        if attributes.iter().any(|a| matches!(a, Attr::Skip)) {
            continue;
        }

        let path = match &field.ty {
            Type::Path(path) => quote! { #path },
            Type::Array(array) => quote! { #array },
            Type::Tuple(tuple) => quote! { #tuple },
            _ => {
                return Err(syn::Error::new_spanned(
                    field,
                    "Only named fields are supported, use #[config(skip)] to skip a field",
                ))
            }
        };

        let comment = get_doc_comment(&field.attrs)
            .map(|c| quote! { Some(#c) })
            .unwrap_or_else(|| quote! { None });

        let field_env_attr = attributes
            .iter()
            .find_map(|a| if let Attr::Env(e) = a { Some(e) } else { None })
            .or(struct_env_attr);

        let field_cli_attr = attributes
            .iter()
            .find_map(|a| if let Attr::Cli(e) = a { Some(e) } else { None })
            .or(struct_cli_attr);

        let transform_attr = attributes
            .iter()
            .find_map(|a| {
                if let Attr::KeyType(e) = a {
                    Some(e)
                } else {
                    None
                }
            })
            .map(|e| {
                quote! { |path: &::config::KeyPath, value: ::config::Value| {
                    ::config::transform_from_graph(path, &#e, value)
                }}
            })
            .unwrap_or_else(|| quote! { <#path as ::config::Config>::transform });

        let type_attr = attributes
            .iter()
            .find_map(|a| {
                if let Attr::KeyType(e) = a {
                    Some(e)
                } else {
                    None
                }
            })
            .map(|t| quote! { #t })
            .unwrap_or_else(|| quote! { <#path as ::config::Config>::graph() });

        let add_attrs = {
            let env_attr = field_env_attr.and_then(|env_attr| {
                env_attr
                    .skip
                    .then_some(quote! { let key = key.with_skip_env(); })
            });

            let cli_attr = field_cli_attr.and_then(|cli_attr| {
                cli_attr
                    .skip
                    .then_some(quote! { let key = key.with_skip_cli(); })
            });

            quote! {
                let key = ::config::Key::new(#type_attr);
                #env_attr
                #cli_attr
                let key = key.with_transformer(#transform_attr);
                key.with_comment(#comment)
            }
        };

        keys_init.push(quote! {
            keys.insert(stringify!(#ident).to_string(), {
                #add_attrs
            });
        });
    }

    let name = &ast.ident;
    Ok(quote! {
        impl ::config::Config for #name {
            const PKG_NAME: Option<&'static str> = option_env!("CARGO_PKG_NAME");
            const ABOUT: Option<&'static str> = option_env!("CARGO_PKG_DESCRIPTION");
            const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
            const AUTHOR : Option<&'static str> = option_env!("CARGO_PKG_AUTHORS");

            fn graph() -> ::std::sync::Arc<::config::KeyGraph> {
                let builder = ::config::KeyGraph::builder::<Self>();
                if let Some(graph) = builder.get() {
                    return graph;
                }

                let mut keys = ::std::collections::BTreeMap::new();

                #(#keys_init)*

                builder.build(::config::KeyGraph::Struct(keys))
            }
        }
    }
    .into())
}

struct EnvAttr {
    /// Skip this field in the env
    skip: bool,
}

struct CliAttr {
    /// Skip this field in the cli
    skip: bool,
}

enum Attr {
    Skip,
    KeyType(syn::Expr),
    Env(EnvAttr),
    Cli(CliAttr),
}

fn get_attributes(attrs: &[syn::Attribute]) -> syn::Result<Vec<Attr>> {
    attrs
        .iter()
        .filter(|a| a.path().is_ident("config"))
        .map(|attr| {
            // the syntax looks like #[config(default = "true")] or #[config(from_str = "parse")] or #[config(from_str)] or #[config(default)]
            attr.parse_args_with(
                syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
            )
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .map(|meta| {
            match meta {
                syn::Meta::Path(path) => {
                    if path.is_ident("skip") {
                        Ok(Attr::Skip)
                    } else {
                        Err(syn::Error::new_spanned(path, "Unknown attribute"))
                    }
                }
                syn::Meta::NameValue(syn::MetaNameValue { path, value, .. }) => {
                    if path.is_ident("graph") {
                        // Try see if the value is a string literal
                        match value {
                            syn::Expr::Lit(syn::ExprLit {
                                lit: syn::Lit::Str(lit),
                                ..
                            }) => Ok(Attr::KeyType(syn::parse_str(
                                &lit.value(),
                            )?)),
                            expr => Ok(Attr::KeyType(expr)),
                        }
                    } else {
                        Err(syn::Error::new_spanned(path, "Unknown attribute"))
                    }
                }
                syn::Meta::List(list) => {
                    if list.path.is_ident("env") {
                        let meta = list.parse_args_with(
                            syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
                        )?;

                        let mut skip = false;

                        for m in meta {
                            match m {
                                syn::Meta::Path(path) => {
                                    if path.is_ident("skip") {
                                        skip = true;
                                    } else {
                                        return Err(syn::Error::new_spanned(path, "Unknown attribute"));
                                    }
                                }
                                _ => return Err(syn::Error::new_spanned(m, "Unknown attribute")),
                            }
                        }

                        Ok(Attr::Env(EnvAttr { skip }))
                    } else if list.path.is_ident("cli") {
                        let meta = list.parse_args_with(
                            syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
                        )?;

                        let mut skip = false;

                        for m in meta {
                            match m {
                                syn::Meta::Path(path) => {
                                    if path.is_ident("skip") {
                                        skip = true;
                                    } else {
                                        return Err(syn::Error::new_spanned(path, "Unknown attribute"));
                                    }
                                }
                                _ => return Err(syn::Error::new_spanned(m, "Unknown attribute")),
                            }
                        }

                        Ok(Attr::Cli(CliAttr { skip }))
                    } else {
                        Err(syn::Error::new_spanned(list.path, "Unknown attribute"))
                    }
                }
            }
        })
        .collect::<Result<Vec<_>, _>>()
}

fn get_doc_comment(attrs: &[syn::Attribute]) -> Option<String> {
    attrs
        .iter()
        .find(|a| a.path().is_ident("doc"))
        .and_then(|a| {
            if let syn::Meta::NameValue(syn::MetaNameValue {
                value:
                    syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(lit),
                        ..
                    }),
                ..
            }) = &a.meta
            {
                Some(lit.value())
            } else {
                None
            }
        })
}
