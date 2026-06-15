use quote::{ToTokens, quote};
use syn::{Expr, Ident, Token, parse::Parse};
use uuid::Uuid;

pub enum ParsedAdapter {
    Widget(syn::Expr),
    StatefulWidget(syn::Expr, syn::Expr),
}

impl Parse for ParsedAdapter {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let adapter_name: Ident = input.parse()?;
        let content;
        syn::parenthesized!(content in input);

        match adapter_name.to_string().as_str() {
            "widget" => {
                let expr: Expr = content.parse()?;
                if !content.is_empty() {
                    return Err(content.error("`widget(...)` expects exactly one expression"));
                }
                Ok(ParsedAdapter::Widget(expr))
            }
            "stateful" => {
                let expr: Expr = content.parse()?;
                content.parse::<Token![,]>()?;
                let state: Expr = content.parse()?;
                if !content.is_empty() {
                    return Err(content.error("`stateful(...)` expects exactly `widget, state`"));
                }
                Ok(ParsedAdapter::StatefulWidget(expr, state))
            }
            _ => Err(syn::Error::new(
                adapter_name.span(),
                "expected `widget(...)` or `stateful(widget, state)`",
            )),
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
                            key: ::ratatui_kit::ElementKey::decl(#decl_key),
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
                            key: ::ratatui_kit::ElementKey::decl(#decl_key),
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
