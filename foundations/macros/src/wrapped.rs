use proc_macro::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::{parse_quote, FnArg, Ident, ItemFn, Pat, Visibility};

pub fn wrapped(args: TokenStream, input: TokenStream) -> syn::Result<proc_macro2::TokenStream> {
    let func_name = syn::parse::<Ident>(args)?;
    let mut input = syn::parse::<ItemFn>(input)?;

    let mut has_self_argument = false;
    // remove types from args for use when calling the inner function
    let mut args_without_types = vec![];
    let mut args_without_types_including_self = vec![];
    for arg in &input.sig.inputs {
        match arg {
            FnArg::Receiver(_) => {
                has_self_argument = true;
                args_without_types_including_self.push(quote!(self));
            }
            FnArg::Typed(arg) => {
                let tokens = if let Pat::Ident(mut a) = *arg.pat.clone() {
                    a.attrs.clear();
                    a.mutability = None;
                    a.into_token_stream()
                } else {
                    arg.pat.clone().into_token_stream()
                };
                args_without_types.push(tokens.clone());
                args_without_types_including_self.push(tokens);
            }
        }
    }

    let self_dot = if has_self_argument {
        quote!(self.)
    } else {
        quote!()
    };

    let asyncness_await = match input.sig.asyncness {
        Some(_) => quote!(.await),
        None => quote!(),
    };

    let attrs = input.attrs.clone();
    let vis = input.vis.clone();
    let sig = input.sig.clone();

    let orig_name = input.sig.ident.clone();
    let inner_name = format_ident!("_wrappped_inner_{orig_name}");

    input.sig.ident = inner_name.clone();
    input.vis = Visibility::Inherited; // make sure the inner function isn't leaked to the public
    input.attrs = vec![
        // we will put the original attributes on the function we make
        // we also don't want the inner function to appear in docs or autocomplete (if they do, they should be deprecated and give a warning if they are used)
        parse_quote!(#[doc(hidden)]),
        parse_quote!(#[deprecated = "internal wrapper function, please do not use!"]),
        parse_quote!(#[inline(always)]), // let's make sure we don't produce more overhead than we need to, the output should produce similar assembly to the input (besides the end)
    ];

    // for functions that take a self argument, we will need to put the inner function outside of our new function since we don't know what type self is
    let (outer_input, inner_input) = if has_self_argument {
        (Some(input), None)
    } else {
        (None, Some(input))
    };

    Ok(quote! {
        #outer_input

        #(#attrs)* #vis #sig {
            #inner_input

            #[allow(deprecated)]
            #func_name(#self_dot #inner_name(#(#args_without_types),*) #asyncness_await)
        }
    })
}
