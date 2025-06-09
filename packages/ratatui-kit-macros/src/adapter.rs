use quote::{ToTokens, quote};
use syn::{Expr, parse::Parse, spanned::Spanned};
use uuid::Uuid;

pub struct ParsedAdapter {
    pub expr: syn::Expr,
}

impl Parse for ParsedAdapter {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let expr: Expr = input.parse()?;
        match expr {
            Expr::Path(_) => Ok(Self { expr }),
            Expr::MethodCall(_) => Ok(Self { expr }),
            _ => Err(syn::Error::new(
                expr.span(),
                "Expected a path or method call expression for ratatui widget",
            )),
        }
    }
}

impl ToTokens for ParsedAdapter {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let decl_key = Uuid::new_v4().as_u128();
        let expr = &self.expr;

        tokens.extend(quote! {
            {
                let mut _element=::ratatui_kit::Element::<::ratatui_kit::components::Adapter>{
                    key: ::ratatui_kit::ElementKey::new(#decl_key),
                    props: ::ratatui_kit::components::AdapterProps(Some(std::sync::Arc::new(#expr))),
                };
                _element
            }
        });
    }
}
