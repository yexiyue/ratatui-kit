use quote::{ToTokens, quote};
use syn::{
    LitStr, Token,
    parse::Parse,
    punctuated::Punctuated,
    token::{Brace, Comma},
};

use crate::element::ParsedElementHead;

pub struct ParsedRoute {
    pub path: LitStr,
    // 复用 `element!` 的头部解析:`Component` 或 `Component(prop: val)`。
    // 持有不含 children 的 `ParsedElementHead`——路由组件结构上无静态 children,
    // `{}` 留给本结构体的 `children` 当嵌套子路由。
    pub element: ParsedElementHead,
    pub children: Routes,
}

#[derive(Default)]
pub struct Routes(pub Punctuated<ParsedRoute, Comma>);

impl Parse for ParsedRoute {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let path: LitStr = input.parse()?;
        input.parse::<Token![=>]>()?;
        // ParsedElementHead 只吃 `Ty` + 可选 `(props)`,不消费 `{}`——后者归子路由。
        let element = input.parse::<ParsedElementHead>()?;

        // 路由身份由 path 决定,`key:` 在路由元素上无意义,显式拒绝以免误用。
        if let Some(span) = element.key_span() {
            return Err(syn::Error::new(
                span,
                "路由组件不支持 `key:`,路由身份由 path 决定",
            ));
        }

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
        let children = &self.children;
        // head 的 to_element_expr 输出带括号的 element 表达式,且传入空 children——
        // 路由组件无静态 children。router 直接 `.into_any()`,不依赖其 token 形状。
        let element = self.element.to_element_expr(&[]);

        // 经 Route::new 构造,使含动态参数的路由在构造期一次性编译匹配正则
        // (Route 的 matcher 字段私有,不能用结构体字面量构造)。
        tokens.extend(quote! {
            ::ratatui_kit::components::Route::new(
                #path.to_string(),
                #element.into_any(),
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
