use proc_macro::TokenStream;

mod bootstrap;
mod helpers;
mod metrics;
mod settings;
mod wrapped;

/// #[wrapped(inspect_output)]
/// async fn respond(req: Request<Incoming>) -> Result<Response<Body>,
/// Infallible> { todo!() }
///
///
/// async fn respond(req: Request<Incoming>) -> Result<Response<Body>,
/// Infallible> {     async fn __inspect_output(req: Request<Incoming>) ->
/// Result<Response<Body>, Infallible> { todo!() }
///     inspect_output(__wrapped(req).await)
/// }
#[proc_macro_attribute]
pub fn wrapped(args: TokenStream, input: TokenStream) -> TokenStream {
	handle_error(wrapped::wrapped(args, input))
}

/// #[settings]
/// struct Config {
///     /// The log level to use.
///     level: String,
/// }
///
/// ===
///
/// #[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
/// pub struct Configuration {
///    /// The log level to use.
///    pub level: String,
/// }
#[proc_macro_attribute]
pub fn auto_settings(args: TokenStream, input: TokenStream) -> TokenStream {
	handle_error(settings::settings(args, input))
}

#[proc_macro_derive(Settings, attributes(settings))]
pub fn derive_settings(input: TokenStream) -> TokenStream {
	handle_error(settings::derive_settings(input))
}

#[proc_macro_attribute]
pub fn metrics(args: TokenStream, input: TokenStream) -> TokenStream {
	handle_error(metrics::metrics(args, input))
}

#[proc_macro_attribute]
pub fn bootstrap(args: TokenStream, input: TokenStream) -> TokenStream {
	handle_error(bootstrap::bootstrap(args, input))
}

fn handle_error<T: Into<TokenStream>>(result: Result<T, syn::Error>) -> TokenStream {
	match result {
		Ok(tokens) => tokens.into(),
		Err(err) => err.to_compile_error().into(),
	}
}
