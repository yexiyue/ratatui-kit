use quote::{ToTokens, quote};
use syn::{Expr, Token, parse::Parse};
use uuid::Uuid;

pub enum ParsedAdapter {
    Widget(syn::Expr),
    StatefulWidget(syn::Expr, syn::Ident),
}

impl Parse for ParsedAdapter {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.peek(syn::token::Paren) {
            let content;
            syn::parenthesized!(content in input);
            let expr: Expr = content.parse()?;
            content.parse::<Token![,]>()?;
            let state_ident: syn::Ident = content.parse()?;
            Ok(ParsedAdapter::StatefulWidget(expr, state_ident))
        } else {
            let expr: Expr = input.parse()?;
            Ok(ParsedAdapter::Widget(expr))
        }
    }
}

impl ToTokens for ParsedAdapter {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let decl_key = Uuid::new_v4().as_u128();
        match self {
            ParsedAdapter::Widget(expr) => {
                tokens.extend(quote! {
                    {
                        let mut _element=::ratatui_kit::Element::<::ratatui_kit::components::WidgetAdapter<_>>{
                            key: ::ratatui_kit::ElementKey::new(#decl_key),
                            props: ::ratatui_kit::components::WidgetAdapterProps{
                                inner: #expr
                            },
                        };
                        _element
                    }
                });
            }
            ParsedAdapter::StatefulWidget(expr, state_ident) => {
                tokens.extend(quote! {
                    {
                        let mut _element=::ratatui_kit::Element::<::ratatui_kit::components::StatefulWidgetAdapter<_>>{
                            key: ::ratatui_kit::ElementKey::new(#decl_key),
                            props: ::ratatui_kit::components::StatefulWidgetAdapterProps{
                                inner: #expr,
                                state: #state_ident
                            },
                        };
                        _element
                    }
                });
            }
        }
    }
}
