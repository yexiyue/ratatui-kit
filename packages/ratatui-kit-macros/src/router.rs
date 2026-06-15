use quote::{ToTokens, quote};
use syn::{
    LitStr, Token,
    parse::Parse,
    punctuated::Punctuated,
    token::{Brace, Comma},
};

use crate::element::ParsedElement;

pub struct ParsedRoute {
    pub path: LitStr,
    /// 复用 `element!` 的头部解析:`Component` 或 `Component(prop: val)`。
    /// 只持有「类型 + props」(经 `ParsedElement::parse_head`,不含 children)——
    /// `{}` 留给本结构体的 `children` 当嵌套子路由。
    pub element: ParsedElement,
    pub children: Routes,
}

#[derive(Default)]
pub struct Routes(pub Punctuated<ParsedRoute, Comma>);

impl Parse for ParsedRoute {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let path: LitStr = input.parse()?;
        input.parse::<Token![=>]>()?;
        // parse_head 只吃 `Ty` + 可选 `(props)`,不消费 `{}`——后者归子路由。
        let element = ParsedElement::parse_head(input)?;

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
        let element = &self.element;
        let children = &self.children;

        // 经 Route::new 构造,使含动态参数的路由在构造期一次性编译匹配正则
        // (Route 的 matcher 字段私有,不能用结构体字面量构造)。
        //
        // `element` 是 `ParsedElement`,其 `ToTokens` 在 children 为空时产出等价于
        // `element!(Comp(props))` 的 `Element` 构造块。外层括号 load-bearing——
        // 块表达式 `{ ... }` 须加括号才能在实参位继续 `.into_any()`。
        tokens.extend(quote! {
            ::ratatui_kit::components::Route::new(
                #path.to_string(),
                (#element).into_any(),
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
