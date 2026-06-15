use quote::{ToTokens, quote};
use syn::{
    LitStr, Token, TypePath,
    parse::Parse,
    punctuated::Punctuated,
    token::{Brace, Comma},
};

pub struct ParsedRoute {
    pub path: LitStr,
    pub element: TypePath,
    pub children: Routes,
}

#[derive(Default)]
pub struct Routes(pub Punctuated<ParsedRoute, Comma>);

impl Parse for ParsedRoute {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let path: LitStr = input.parse()?;
        input.parse::<Token![=>]>()?;
        let element: TypePath = input.parse()?;

        let mut children = Routes::default();
        if input.peek(Brace) {
            let children_input;
            syn::braced!(children_input in input);
            children = children_input.parse()?;
        }

        Ok(ParsedRoute {
            path,
            element,
            children,
        })
    }
}

impl Parse for Routes {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let routes = Punctuated::parse_terminated(input)?;
        Ok(Routes(routes))
    }
}

impl ToTokens for ParsedRoute {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let path = &self.path;
        let element = &self.element;
        let children = &self.children;

        // 经 Route::new 构造,使含动态参数的路由在构造期一次性编译匹配正则
        // (Route 的 matcher 字段私有,不能用结构体字面量构造)。
        tokens.extend(quote! {
            ::ratatui_kit::components::Route::new(
                #path.to_string(),
                ::ratatui_kit::element!(#element).into_any(),
                #children.into(),
            )
        });
    }
}

impl ToTokens for Routes {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let routes = self.0.iter().map(|route| route.to_token_stream());

        tokens.extend(quote! {
            vec![
                #(#routes),*
            ]
        });
    }
}
