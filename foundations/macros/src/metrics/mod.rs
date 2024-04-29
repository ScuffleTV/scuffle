use darling::{ast::NestedMeta, FromMeta};
use quote::ToTokens;
use syn::{parse::Parse, punctuated::Punctuated, spanned::Spanned, Token};

#[derive(Debug, FromMeta)]
#[darling(default)]
struct Options {
    crate_path: Option<syn::Path>,
    optional: bool,
    builder: Option<syn::Expr>,
}

impl Options {
    fn from_attrs(attrs: &[syn::Attribute]) -> syn::Result<Self> {
        let mut optional = false;
        let mut builder = None;

        for attr in attrs {
            match &attr.meta {
                syn::Meta::Path(path) => {
                    if path.is_ident("optional") {
                        if optional {
                            return Err(syn::Error::new_spanned(attr, "duplicate attribute"));
                        }

                        optional = true;
                    }
                }
                syn::Meta::NameValue(syn::MetaNameValue { path, value, .. }) => {
                    if path.is_ident("builder") {
                        if builder.is_some() {
                            return Err(syn::Error::new_spanned(attr, "duplicate attribute"));
                        }

                        builder = Some(value.clone());
                    }
                }
                _ => return Err(syn::Error::new_spanned(attr, "unexpected attribute")),
            }
        }

        Ok(Options {
            crate_path: None,
            optional,
            builder,
        })
    }
}

impl Default for Options {
    fn default() -> Self {
        Options {
            crate_path: None,
            optional: false,
            builder: None,
        }
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

enum ModuleItem {
    Other(syn::Item),
    Function(proc_macro2::TokenStream),
}

struct FunctionAttrs {
    cfg_attrs: Vec<syn::Attribute>,
    docs: Vec<syn::LitStr>,
    options: Options,
}

impl FunctionAttrs {
    fn from_attrs(attrs: Vec<syn::Attribute>) -> syn::Result<Self> {
        let (cfg_attrs, others): (Vec<_>, Vec<_>) = attrs
            .into_iter()
            .partition(|attr| attr.path().is_ident("cfg"));

        let (doc_attrs, others): (Vec<_>, Vec<_>) = others
            .into_iter()
            .partition(|attr| attr.path().is_ident("doc"));

        Ok(FunctionAttrs {
            cfg_attrs,
            docs: doc_attrs
                .into_iter()
                .map(|attr| match attr.meta {
                    syn::Meta::NameValue(syn::MetaNameValue {
                        value:
                            syn::Expr::Lit(syn::ExprLit {
                                lit: syn::Lit::Str(lit),
                                ..
                            }),
                        ..
                    }) => Ok(lit),
                    _ => Err(syn::Error::new_spanned(attr, "expected string literal")),
                })
                .collect::<Result<_, _>>()?,
            options: Options::from_attrs(&others)?,
        })
    }
}

pub struct Function {
    vis: syn::Visibility,
    fn_token: Token![fn],
    ident: syn::Ident,
    args: syn::punctuated::Punctuated<FnArg, Token![,]>,
    arrow_token: Token![->],
    ret: syn::Type,
    attrs: FunctionAttrs,
}

impl Parse for Function {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let attrs = input.call(syn::Attribute::parse_outer)?;
        let vis = input.parse()?;
        let fn_token = input.parse()?;
        let ident = input.parse()?;
        let args_content;
        let _paren = syn::parenthesized!(args_content in input);
        let args = args_content.parse_terminated(FnArg::parse, Token![,])?;
        let arrow_token = input.parse()?;
        let ret = input.parse()?;
        input.parse::<Token![;]>()?;

        Ok(Function {
            vis,
            fn_token,
            ident,
            args,
            arrow_token,
            ret,
            attrs: FunctionAttrs::from_attrs(attrs)?,
        })
    }
}

struct FnArg {
    cfg_attrs: Vec<syn::Attribute>,
    other_attrs: Vec<syn::Attribute>,
    ident: syn::Ident,
    colon_token: Token![:],
    ty: syn::Type,
    struct_ty: StructTy,
}

enum StructTy {
    Clone(syn::Type),
    Into(syn::Type),
    Raw(syn::Type),
    Str(syn::Type),
}

impl StructTy {
    fn ty(&self) -> &syn::Type {
        match self {
            StructTy::Clone(ty) => ty,
            StructTy::Into(ty) => ty,
            StructTy::Raw(ty) => ty,
            StructTy::Str(ty) => ty,
        }
    }
}

fn type_to_struct_type(ty: syn::Type) -> syn::Result<StructTy> {
    match ty.clone() {
        syn::Type::Reference(syn::TypeReference { elem, lifetime, .. }) => {
            if lifetime.map_or(false, |lifetime| lifetime.ident == "static") {
                return Ok(StructTy::Raw(ty));
            }

            if let syn::Type::Path(syn::TypePath { path, .. }) = &*elem {
                if path.is_ident("str") {
                    return Ok(StructTy::Str(
                        syn::parse_quote_spanned! { ty.span() => ::std::sync::Arc<#path> },
                    ));
                }
            }

            Ok(StructTy::Clone(*elem))
        }
        // Also support impl types
        syn::Type::ImplTrait(impl_trait) => impl_trait
            .bounds
            .iter()
            .find_map(|bound| match bound {
                syn::TypeParamBound::Trait(syn::TraitBound {
                    path: syn::Path { segments, .. },
                    ..
                }) => {
                    let first_segment = segments.first()?;
                    if first_segment.ident != "Into" {
                        return None;
                    }

                    let args = match first_segment.arguments {
                        syn::PathArguments::AngleBracketed(ref args) => args.args.clone(),
                        _ => return None,
                    };

                    if args.len() != 1 {
                        return None;
                    }

                    match &args[0] {
                        syn::GenericArgument::Type(ty) => Some(StructTy::Into(ty.clone())),
                        _ => None,
                    }
                }
                _ => None,
            })
            .ok_or_else(|| {
                syn::Error::new_spanned(impl_trait, "only impl Into<T> trait bounds are supported")
            }),
        _ => Ok(StructTy::Raw(ty)),
    }
}

impl Parse for FnArg {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let attrs = input.call(syn::Attribute::parse_outer)?;
        let ident = input.parse()?;
        let colon_token = input.parse()?;
        let ty: syn::Type = input.parse()?;
        let struct_ty = type_to_struct_type(ty.clone())?;

        let (cfg_attrs, other_attrs): (Vec<_>, Vec<_>) = attrs
            .into_iter()
            .partition(|attr| attr.path().is_ident("cfg"));

        Ok(FnArg {
            ident,
            cfg_attrs,
            other_attrs,
            colon_token,
            ty,
            struct_ty,
        })
    }
}

impl ToTokens for FnArg {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        for attr in &self.cfg_attrs {
            attr.to_tokens(tokens);
        }

        self.ident.to_tokens(tokens);
        self.colon_token.to_tokens(tokens);
        self.ty.to_tokens(tokens);
    }
}

fn metric_function(
    input: proc_macro2::TokenStream,
    module_name: Option<&syn::Ident>,
    options: &Options,
) -> syn::Result<proc_macro2::TokenStream> {
    let item = syn::parse2::<Function>(input)?;

    let crate_path = &options
        .crate_path
        .clone()
        .unwrap_or_else(|| syn::parse_quote!(scuffle_foundations));

    let ident = &item.ident;
    let vis = &item.vis;
    let ret = &item.ret;

    let const_assert_ret = quote::quote_spanned! { ret.span() =>
        __assert_impl_collector::<#ret>();
    };

    let const_assert_args = item.args.iter().map(|arg| {
        let ty = arg.struct_ty.ty();
        quote::quote_spanned! { ty.span() =>
            __assert_arg_static::<#ty>();
        }
    });

    let attrs = &item.attrs.cfg_attrs;
    let docs = &item.attrs.docs;
    let fn_token = &item.fn_token;
    let arrow_token = &item.arrow_token;
    let args = &item.args;

    let struct_fields = args
        .iter()
        .map(|arg| {
            let ident = &arg.ident;
            let ty = &arg.struct_ty.ty();
            let attrs = &arg.other_attrs;

            Ok(quote::quote! {
                #(#attrs)*
                #ident: #ty
            })
        })
        .collect::<syn::Result<Vec<_>>>()?;

    let has_args = !struct_fields.is_empty();

    let build_args = args
        .iter()
        .map(|arg| {
            let ident = &arg.ident;
            let arg = match &arg.struct_ty {
                StructTy::Clone(_) => quote::quote! {
                    ::core::clone::Clone::clone(#ident)
                },
                StructTy::Into(_) => quote::quote! {
                    ::core::convert::Into::into(#ident)
                },
                StructTy::Raw(_) => quote::quote! {
                    #ident
                },
                StructTy::Str(_) => quote::quote! {
                    ::std::sync::Arc::from(#ident)
                },
            };

            Ok(quote::quote! {
                #ident: #arg
            })
        })
        .collect::<syn::Result<Vec<_>>>()?;

    let metric_ident = if has_args {
        quote::quote! {
            #crate_path::telementry::metrics::serde::Family<__Args, #ret>
        }
    } else {
        quote::quote! {
            #ret
        }
    };

    let make_metric = {
        let make_metric = if let Some(builder) = item
            .attrs
            .options
            .builder
            .as_ref()
            .or(options.builder.as_ref())
        {
            let constructor = quote::quote! {
                {
                    (|| {
                        #crate_path::telementry::metrics::MetricBuilder::build(&#builder)
                    }) as fn() -> #ret
                }
            };

            if has_args {
                quote::quote! {
                    #crate_path::telementry::metrics::serde::Family::new_with_constructor(#constructor)
                }
            } else {
                quote::quote! {
                    #constructor()
                }
            }
        } else {
            if has_args {
                quote::quote! {
                    #crate_path::telementry::metrics::serde::Family::default()
                }
            } else {
                quote::quote! {
                    Default::default()
                }
            }
        };

        let registry = if item.attrs.options.optional || options.optional {
            quote::quote! {
                #crate_path::telementry::metrics::registries::Registries::get_optional_sub_registry(stringify!(#module_name))
            }
        } else {
            quote::quote! {
                #crate_path::telementry::metrics::registries::Registries::get_main_sub_registry(stringify!(#module_name))
            }
        };

        let help = if docs.is_empty() {
            quote::quote! {
                "No documentation provided"
            }
        } else {
            quote::quote! {
                ::core::primitive::str::trim_end_matches(&[
                    #(::core::primitive::str::trim(#docs)),*
                ].join(" "), ".")
            }
        };

        quote::quote! {
            let metric = #make_metric;

            #registry.register(
                stringify!(#ident),
                #help,
                ::std::clone::Clone::clone(&metric),
            );

            return metric;
        }
    };

    let serde_path_str = format!(
        "{}",
        quote::quote! {
            #crate_path::macro_reexports::serde
        }
    );

    let args_struct = if has_args {
        quote::quote! {
            #[derive(::std::fmt::Debug, ::std::clone::Clone, PartialEq, Eq, Hash, #crate_path::macro_reexports::serde::Serialize)]
            #[serde(crate = #serde_path_str)]
            struct __Args {
                #(#struct_fields,)*
            }
        }
    } else {
        quote::quote! {}
    };

    let assert_collector_fn = quote::quote! {
        const fn __assert_impl_collector<T: #crate_path::macro_reexports::prometheus_client::metrics::TypedMetric>() {}
    };
    let assert_arg_fn = if has_args {
        quote::quote! {
            const fn __assert_arg_static<T: 'static>() {}
        }
    } else {
        quote::quote! {}
    };

    let get_metric = if has_args {
        quote::quote! {
            ::std::clone::Clone::clone(&__METRIC.get_or_create(&__Args {
                #(#build_args,)*
            }))
        }
    } else {
        quote::quote! {
            ::std::clone::Clone::clone(&__METRIC)
        }
    };

    let fn_body = quote::quote! {
        #args_struct
        #assert_collector_fn
        #assert_arg_fn
        #const_assert_ret
        #(#const_assert_args)*

        static __METRIC: #crate_path::macro_reexports::once_cell::sync::Lazy<#metric_ident> = #crate_path::macro_reexports::once_cell::sync::Lazy::new(|| {
            #make_metric;
        });

        #get_metric
    };

    Ok(quote::quote! {
        #(#attrs)*
        #(#[doc = #docs])*
        #[must_use]
        #vis #fn_token #ident(#args) #arrow_token #ret {
            #fn_body
        }
    })
}

pub fn metrics(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> syn::Result<proc_macro2::TokenStream> {
    let args = syn::parse::<Options>(args)?;

    let module = match syn::parse::<syn::Item>(input)? {
        syn::Item::Mod(module) => module,
        syn::Item::Verbatim(tokens) => return metric_function(tokens, None, &args),
        item => {
            return Err(syn::Error::new_spanned(
                item,
                "expected module or bare function",
            ))
        }
    };

    if args.builder.is_some() {
        return Err(syn::Error::new_spanned(
            args.builder.as_ref().unwrap(),
            "builder attribute is only allowed on functions",
        ));
    }

    let module_name = &module.ident;
    let vis = &module.vis;

    let items = module
        .content
        .into_iter()
        .flat_map(|(_, item)| item)
        .map(|item| match item {
            syn::Item::Verbatim(verbatim) => {
                metric_function(verbatim, Some(module_name), &args).map(ModuleItem::Function)
            }
            item => Ok(ModuleItem::Other(item)),
        })
        .collect::<syn::Result<Vec<_>>>()?;

    let items = items.into_iter().map(|item| match item {
        ModuleItem::Other(item) => item,
        ModuleItem::Function(item) => syn::Item::Verbatim(item),
    });

    Ok(quote::quote! {
        #vis mod #module_name {
            #(#items)*
        }
    })
}
