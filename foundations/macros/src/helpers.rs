/// #[doc = "docs"] or /// docs
pub fn parse_docs(attr: &[syn::Attribute]) -> Vec<syn::LitStr> {
	attr.iter()
		.filter(|attr| attr.path().is_ident("doc"))
		.filter_map(|attr| match &attr.meta {
			syn::Meta::NameValue(meta) => {
				if let syn::Expr::Lit(syn::ExprLit {
					lit: syn::Lit::Str(lit), ..
				}) = &meta.value
				{
					Some(syn::LitStr::new(lit.value().trim(), lit.span()))
				} else {
					None
				}
			}
			_ => None,
		})
		.collect()
}
