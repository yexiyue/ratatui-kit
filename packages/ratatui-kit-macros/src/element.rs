use quote::{ToTokens, quote};
use syn::{
    Expr, FieldValue, Member, Token, TypePath, braced, parse::Parse, punctuated::Punctuated,
    token::Comma,
};
use uuid::Uuid;

use crate::adapter::ParsedAdapter;

enum ParsedElementChild {
    Element(ElementOrAdapter),
    Expr(Expr),
}

pub struct ParsedElement {
    ty: TypePath,
    props: Punctuated<FieldValue, Comma>,
    children: Vec<ParsedElementChild>,
}

impl Parse for ParsedElement {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ty: TypePath = input.parse()?;
        let props = if input.peek(syn::token::Paren) {
            let props_input;
            syn::parenthesized!(props_input in input);
            Punctuated::parse_terminated(&props_input)?
        } else {
            Punctuated::new()
        };

        let mut children = Vec::new();

        if input.peek(syn::token::Brace) {
            let children_input;
            braced!(children_input in input);
            while !children_input.is_empty() {
                if children_input.peek(Token![#]) {
                    children_input.parse::<Token![#]>()?;
                    let expr;
                    syn::parenthesized!(expr in children_input);
                    children.push(ParsedElementChild::Expr(expr.parse()?));
                } else {
                    children.push(ParsedElementChild::Element(children_input.parse()?));
                }
            }
        }

        Ok(Self {
            ty,
            props,
            children,
        })
    }
}

impl ToTokens for ParsedElement {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let ty = &self.ty;
        let decl_key = Uuid::new_v4().as_u128();
        let key = self
            .props
            .iter()
            .find_map(|FieldValue { member, expr, .. }| match member {
                Member::Named(ident) if ident == "key" => Some(quote!((#decl_key,#expr))),
                _ => None,
            })
            .unwrap_or_else(|| quote!(#decl_key));

        let props_assignments = self
            .props
            .iter()
            .filter_map(|FieldValue { member, expr, .. }| {
                match member {
                    Member::Named(ident) if ident == "key" => None, // key 已经处理过了
                    _ => Some(quote!(_props.#member = #expr)),
                }
            })
            .collect::<Vec<_>>();

        let set_children = if !self.children.is_empty() {
            let children = self.children.iter().map(|child| match child {
                ParsedElementChild::Expr(expr) => quote!(#expr),
                ParsedElementChild::Element(element) => quote!(#element),
            });
            Some(quote! {
                #(::ratatui_kit::extend_with_elements(&mut _element.props.children,#children);)*
            })
        } else {
            None
        };

        tokens.extend(quote! {
            {
                type Props<'a>= <#ty as ::ratatui_kit::ElementType>::Props<'a>;
                let mut _props = Props::default();
                #(#props_assignments;)*
                let mut _element=::ratatui_kit::Element::<#ty>{
                    key: ::ratatui_kit::ElementKey::new(#key),
                    props: _props,
                };
                #set_children
                _element
            }
        });
    }
}

pub enum ElementOrAdapter {
    Element(ParsedElement),
    Adapter(ParsedAdapter),
}

impl Parse for ElementOrAdapter {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.peek(Token![$]) {
            input.parse::<Token![$]>()?;
            let adapter: ParsedAdapter = input.parse()?;
            Ok(ElementOrAdapter::Adapter(adapter))
        } else {
            let element: ParsedElement = input.parse()?;
            Ok(ElementOrAdapter::Element(element))
        }
    }
}

impl ToTokens for ElementOrAdapter {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            ElementOrAdapter::Element(element) => element.to_tokens(tokens),
            ElementOrAdapter::Adapter(adapter) => adapter.to_tokens(tokens),
        }
    }
}
