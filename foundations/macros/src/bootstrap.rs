use darling::ast::NestedMeta;
use darling::FromMeta;
use proc_macro::TokenStream;
use syn::punctuated::Punctuated;
use syn::{parse_quote, Token};

#[derive(FromMeta)]
#[darling(default)]
struct BootstrapArgs {
	crate_name: syn::Path,
}

impl Default for BootstrapArgs {
	fn default() -> Self {
		BootstrapArgs {
			crate_name: parse_quote!(::scuffle_foundations),
		}
	}
}

impl syn::parse::Parse for BootstrapArgs {
	fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
		if input.is_empty() {
			Ok(BootstrapArgs::default())
		} else {
			let meta_list = Punctuated::<NestedMeta, Token![,]>::parse_terminated(input)?
				.into_iter()
				.collect::<Vec<_>>();
			Ok(BootstrapArgs::from_list(&meta_list)?)
		}
	}
}

/// #[bootstrap(Type, crate = "crate_name")]
pub fn bootstrap(args: TokenStream, input: TokenStream) -> syn::Result<proc_macro2::TokenStream> {
	let args = syn::parse::<BootstrapArgs>(args)?;
	let input = syn::parse::<syn::ItemFn>(input)?;

	let name = &input.sig.ident;

	// The main function should have a single argument
	if input.sig.inputs.len() != 1 {
		return Err(syn::Error::new_spanned(
			input.sig.ident,
			"bootstrap function must have a single argument",
		));
	}

	// Main should be async
	if input.sig.asyncness.is_none() {
		return Err(syn::Error::new_spanned(
			input.sig.fn_token,
			"bootstrap function must be async",
		));
	}

	let ret = &input.sig.output;

	// Sometimes the return value will not be specified
	let call_fn = match ret {
		syn::ReturnType::Default => quote::quote! { |settings| async move {
			#name(settings).await;
			Ok(())
		} },
		_ => quote::quote! { #name },
	};

	let handle_result = match ret {
		syn::ReturnType::Default => quote::quote! { .unwrap(); },
		_ => quote::quote! {},
	};

	let crate_name = &args.crate_name;

	let cfg_attrs = input.attrs.iter().filter(|attr| attr.path().is_ident("cfg"));

	Ok(quote::quote! {
		#(#cfg_attrs)*
		fn #name() #ret {
			#input

			#crate_name::bootstrap::bootstrap(&::std::default::Default::default(), #crate_name::service_info!(), #call_fn)#handle_result
		}
	})
}
