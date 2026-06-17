use proc_macro2::Span;
use quote::{ToTokens, quote};
use syn::{
    Expr, FieldValue, Ident, Member, Pat, Token, TypePath, braced, parse::Parse,
    parse::ParseStream, punctuated::Punctuated, spanned::Spanned, token::Comma,
};
use uuid::Uuid;

use crate::adapter::ParsedAdapter;

// 单个子节点：嵌套元素 / adapter / 任意表达式、或一等控制流(if/for/match)。
//
// `pub(crate)`：`ParsedElementHead::to_element_expr` 以 `&[ParsedElementChild]` 接收
// children 参数,该方法对 `router.rs` 可见(`pub(crate)`),故本类型也需 crate 级可见。
pub(crate) enum ParsedElementChild {
    Element(ElementOrAdapter),
    Expr(Expr),
    // ControlFlow 装箱:If/For 内联持有 syn 的 Expr/Pat,是本枚举最大的变体,
    // 不装箱会触发 clippy::large_enum_variant(parse AST,装箱成本可忽略)。
    ControlFlow(Box<ControlFlow>),
}

// element! 子节点块内的一等控制流。分支体本身又是一组子节点。
//
// 相比把条件渲染塞进表达式插槽,一等控制流让每个分支独立把自己的
// 子节点 `extend` 进 children——故各分支可返回不同元素类型,无需 `.into_any()` 统一类型。
//
// `pub(crate)`:随 [`ParsedElementChild`] 经 `to_element_expr` 的 crate 级签名传染而来。
pub(crate) enum ControlFlow {
    If {
        cond: Expr,
        then_branch: Vec<ParsedElementChild>,
        else_branch: Option<Box<ElseBranch>>,
    },
    For {
        pat: Pat,
        expr: Expr,
        body: Vec<ParsedElementChild>,
    },
    Match {
        expr: Expr,
        arms: Vec<MatchArm>,
    },
}

// `else if ...` 或 `else { ... }`。
pub(crate) enum ElseBranch {
    If(Box<ControlFlow>),
    Block(Vec<ParsedElementChild>),
}

pub(crate) struct MatchArm {
    pat: Pat,
    guard: Option<Expr>,
    body: Vec<ParsedElementChild>,
}

// 解析一段子节点序列(用于元素的 `{}` 块,以及控制流的各分支体)。
fn parse_children(input: ParseStream) -> syn::Result<Vec<ParsedElementChild>> {
    let mut children = Vec::new();
    while !input.is_empty() {
        if input.peek(Token![if]) {
            children.push(ParsedElementChild::ControlFlow(Box::new(parse_if(input)?)));
        } else if input.peek(Token![for]) {
            children.push(ParsedElementChild::ControlFlow(Box::new(parse_for(input)?)));
        } else if input.peek(Token![match]) {
            children.push(ParsedElementChild::ControlFlow(Box::new(parse_match(
                input,
            )?)));
        } else if input.peek(syn::token::Brace) {
            // `{ expr }`:把子节点位置交还给任意 Rust 表达式(返回 Option/Vec/Iterator/Element)。
            children.push(ParsedElementChild::Expr(input.parse()?));
        } else if is_macro_call(input) {
            // 宏调用 `path!(...)`(典型是嵌套的 `element!(...)`,也含 `vec![...]` 等)在子节点位
            // 等同 `{ expr }` embed——整段当 Rust 表达式解析。否则 `element` 会被当成无 props
            // 组件头、剩下的 `!(...)` 在下一轮以 `TypePath` 解析 `!` → 误报 `expected identifier`
            // (这正是把 `Comp(..) { element!(..) }` 误当 children 时最坑的报错)。
            children.push(ParsedElementChild::Expr(input.parse()?));
        } else {
            // 嵌套元素 `Comp(..){..}` 或 `widget(...)` / `stateful(...)` 适配器。
            children.push(ParsedElementChild::Element(input.parse()?));
        }
    }
    Ok(children)
}

// 子节点是否为宏调用 `path!(...)`:fork 出去试解析一个 path,其后紧跟 `!` 即是。
// 组件头 `Comp(props)` 与适配器 `widget(...)` 的 path 之后都是 `(`,不会误判。
fn is_macro_call(input: ParseStream) -> bool {
    let fork = input.fork();
    fork.parse::<syn::Path>().is_ok() && fork.peek(Token![!])
}

fn parse_if(input: ParseStream) -> syn::Result<ControlFlow> {
    input.parse::<Token![if]>()?;
    // parse_without_eager_brace:把后续 `{` 当作分支体起始而非条件表达式的一部分;
    // 同时支持 `if let PAT = EXPR` 这类 let 条件。
    let cond = Expr::parse_without_eager_brace(input)?;
    let content;
    braced!(content in input);
    let then_branch = parse_children(&content)?;

    let else_branch = if input.peek(Token![else]) {
        input.parse::<Token![else]>()?;
        if input.peek(Token![if]) {
            Some(Box::new(ElseBranch::If(Box::new(parse_if(input)?))))
        } else {
            let content;
            braced!(content in input);
            Some(Box::new(ElseBranch::Block(parse_children(&content)?)))
        }
    } else {
        None
    };

    Ok(ControlFlow::If {
        cond,
        then_branch,
        else_branch,
    })
}

fn parse_for(input: ParseStream) -> syn::Result<ControlFlow> {
    input.parse::<Token![for]>()?;
    let pat = Pat::parse_single(input)?;
    input.parse::<Token![in]>()?;
    let expr = Expr::parse_without_eager_brace(input)?;
    let content;
    braced!(content in input);
    let body = parse_children(&content)?;
    Ok(ControlFlow::For { pat, expr, body })
}

fn parse_match(input: ParseStream) -> syn::Result<ControlFlow> {
    input.parse::<Token![match]>()?;
    let expr = Expr::parse_without_eager_brace(input)?;
    let content;
    braced!(content in input);
    let mut arms = Vec::new();
    while !content.is_empty() {
        // 分支模式支持 `A | B`,故用 parse_multi。
        let pat = Pat::parse_multi(&content)?;
        let guard = if content.peek(Token![if]) {
            content.parse::<Token![if]>()?;
            Some(content.parse::<Expr>()?)
        } else {
            None
        };
        content.parse::<Token![=>]>()?;
        // 分支体要求用 `{}` 包裹(里面是一组子节点)。
        let body_content;
        braced!(body_content in content);
        let body = parse_children(&body_content)?;
        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
        }
        arms.push(MatchArm { pat, guard, body });
    }
    Ok(ControlFlow::Match { expr, arms })
}

impl ParsedElementChild {
    // 生成「把本子节点 extend 进 `dest`」的语句。控制流会把内层 extend 包进 if/for/match。
    fn to_extend(&self, dest: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        match self {
            ParsedElementChild::Element(element) => {
                quote!(::ratatui_kit::extend_with_elements(&mut #dest, #element);)
            }
            // Expr 形如块 `{ ... }`:先绑定到局部再 extend——避免把 `{ expr }` 直接做实参
            // 触发 clippy::unnecessary_braces,同时允许块内写多条语句。
            ParsedElementChild::Expr(expr) => quote!({
                let _child = #expr;
                ::ratatui_kit::extend_with_elements(&mut #dest, _child);
            }),
            // ControlFlow 递归把内层 extend 包进 if/for/match 外壳。
            ParsedElementChild::ControlFlow(cf) => cf.to_extend(dest),
        }
    }
}

impl ControlFlow {
    fn to_extend(&self, dest: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        match self {
            ControlFlow::If {
                cond,
                then_branch,
                else_branch,
            } => {
                let then_stmts = then_branch.iter().map(|c| c.to_extend(dest));
                let else_tokens = match else_branch {
                    None => quote!(),
                    Some(b) => match &**b {
                        ElseBranch::Block(children) => {
                            let stmts = children.iter().map(|c| c.to_extend(dest));
                            quote!(else { #(#stmts)* })
                        }
                        // 递归:内层 If 的 to_extend 以 `if ...` 起始,前缀 `else ` 即得 `else if ...`。
                        ElseBranch::If(inner) => {
                            let inner_tokens = inner.to_extend(dest);
                            quote!(else #inner_tokens)
                        }
                    },
                };
                quote!(if #cond { #(#then_stmts)* } #else_tokens)
            }
            ControlFlow::For { pat, expr, body } => {
                let stmts = body.iter().map(|c| c.to_extend(dest));
                quote!(for #pat in #expr { #(#stmts)* })
            }
            ControlFlow::Match { expr, arms } => {
                let arm_tokens = arms.iter().map(|arm| {
                    let MatchArm { pat, guard, body } = arm;
                    let stmts = body.iter().map(|c| c.to_extend(dest));
                    let guard_tokens = match guard {
                        Some(g) => quote!(if #g),
                        None => quote!(),
                    };
                    quote!(#pat #guard_tokens => { #(#stmts)* })
                });
                quote!(match #expr { #(#arm_tokens)* })
            }
        }
    }
}

pub enum PropsItem {
    FieldValue(FieldValue),
    Rest(Expr),
}

impl Parse for PropsItem {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.peek(Token![..]) {
            input.parse::<Token![..]>()?;
            let rest_expr: Expr = input.parse()?;
            Ok(PropsItem::Rest(rest_expr))
        } else {
            let field_value: FieldValue = input.parse()?;
            Ok(PropsItem::FieldValue(field_value))
        }
    }
}

impl ToTokens for PropsItem {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            PropsItem::FieldValue(field_value) => {
                let mut field_value = field_value.clone();
                let expr = &field_value.expr;
                field_value.expr = syn::parse2(quote!((#expr).into())).unwrap();
                tokens.extend(quote!(#field_value))
            }
            PropsItem::Rest(expr) => {
                tokens.extend(quote!(..#expr));
            }
        }
    }
}

impl PropsItem {
    pub fn span(&self) -> Span {
        match self {
            PropsItem::FieldValue(field_value) => field_value.span(),
            PropsItem::Rest(expr) => expr.span(),
        }
    }

    // 若本项是保留的 `key:` 字段(元素身份键),返回其 `FieldValue`。
    //
    // 单一真源:`ToTokens` 的 key 构造与 props 过滤、`ParsedElement::key_span` 都经此查找——
    // 避免「`Member::Named("key")` 匹配 + 魔法串 `"key"`」散落多处、改名时需多处同步。
    fn as_key_field(&self) -> Option<&FieldValue> {
        match self {
            PropsItem::FieldValue(fv) if matches!(&fv.member, Member::Named(ident) if ident == "key") => {
                Some(fv)
            }
            _ => None,
        }
    }
}

// element 的「头部」:类型路径 + 可选 `(props)`,**不含 children**。
//
// 把「头部解析 + element codegen」与「children」在类型上分离——`element!` 与 `routes!`
// 都基于 head 构建,但 `{}` 的归属由各自决定(`element!` 当子节点、`routes!` 当子路由)。
// head 没有 children 字段,故「解析阶段触及 `{}`」在类型层面无法表达,无需注释约定护栏。
pub struct ParsedElementHead {
    ty: TypePath,
    props: Punctuated<PropsItem, Comma>,
}

impl Parse for ParsedElementHead {
    // 只解析类型路径 + 可选 `(props)`。**不 peek/消费 `Brace`**——`{}` 留给调用方。
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ty: TypePath = input.parse()?;
        let props = if input.peek(syn::token::Paren) {
            let props_input;
            syn::parenthesized!(props_input in input);
            Punctuated::parse_terminated(&props_input)?
        } else {
            Punctuated::new()
        };

        let rest_position = props
            .iter()
            .position(|item| matches!(item, PropsItem::Rest(_)));

        if let Some(pos) = rest_position
            && pos != props.len() - 1
        {
            return Err(syn::Error::new(
                props[pos].span(),
                "the rest property must be the last item",
            ));
        }

        Ok(Self { ty, props })
    }
}

impl ParsedElementHead {
    // 返回 `key:` 字段的 span(若存在)。`routes!` 借此拒绝路由元素上的 `key:`——
    // 路由身份由 path 决定,元素 key 在路由场景下无意义(详见 `router.rs`)。
    // 仅 `routes!`(router 特性)调用,故随 router 特性门控,避免无特性时的 dead_code 警告。
    #[cfg(feature = "router")]
    pub fn key_span(&self) -> Option<Span> {
        self.props
            .iter()
            .find_map(PropsItem::as_key_field)
            .map(|fv| fv.member.span())
    }

    // 生成构造 `Element<Ty>` 的表达式 token——element codegen 的**单一真源**。
    //
    // `children` 作为参数注入(而非读取持有状态):`element!` 传实际子节点切片,
    // `routes!` 传空切片。输出**带外层括号**的块表达式 `({ … _element })`,使调用方
    // 可直接 `.into_any()` 或作为实参,无需自己补括号、无需知道内部是块——token 形状
    // 知识收归本模块,`router.rs` 不再依赖它。
    pub(crate) fn to_element_expr(
        &self,
        children: &[ParsedElementChild],
    ) -> proc_macro2::TokenStream {
        let ty = &self.ty;
        let decl_key = Uuid::new_v4().as_u128();
        let has_rest = self
            .props
            .iter()
            .any(|item| matches!(item, PropsItem::Rest(_)));
        // 有用户 `key:` → User((decl_key, expr)) 单次堆分配;否则 Decl(decl_key) 零堆分配。
        let key = self
            .props
            .iter()
            .find_map(PropsItem::as_key_field)
            .map(|fv| {
                let expr = &fv.expr;
                quote!(::ratatui_kit::ElementKey::user((#decl_key, #expr)))
            })
            .unwrap_or_else(|| quote!(::ratatui_kit::ElementKey::decl(#decl_key)));

        let props_assignments = self
            .props
            .iter()
            .filter(|item| item.as_key_field().is_none())
            .map(|props_item| quote!(#props_item))
            .collect::<Vec<_>>();

        let set_children = if !children.is_empty() {
            let dest = quote!(_element.props.children);
            let stmts = children.iter().map(|child| child.to_extend(&dest));
            Some(quote! {
                #(#stmts)*
            })
        } else {
            None
        };

        let has_props_assignments = !props_assignments.is_empty();

        let default_rest = if has_rest {
            quote! {
                #(#props_assignments),*
            }
        } else {
            quote! {
                #(#props_assignments,)*
                ..Default::default()
            }
        };

        let element_binding = if set_children.is_some() {
            quote!(let mut _element=::ratatui_kit::Element::<#ty>{
                key: #key,
                props: _props,
            };)
        } else {
            quote!(let _element=::ratatui_kit::Element::<#ty>{
                key: #key,
                props: _props,
            };)
        };

        // 外层括号 load-bearing:块表达式 `{ … }` 须加括号方能在实参位继续 `.into_any()`。
        if has_props_assignments {
            quote! {
                ({
                    type Props<'a>= <#ty as ::ratatui_kit::ElementType>::Props<'a>;
                    // 用户填满全部字段时,兜底的 `..Default::default()` 会触发 needless_update;
                    // element! 统一以 Default 补未填字段,此处多余属预期(宏无从得知字段总数),显式 allow。
                    #[allow(clippy::needless_update)]
                    let _props = Props{
                        #default_rest
                    };

                    #element_binding
                    #set_children
                    _element
                })
            }
        } else {
            quote! {
                ({
                    type Props<'a>= <#ty as ::ratatui_kit::ElementType>::Props<'a>;
                    let _props = Props::default();
                    #element_binding
                    #set_children
                    _element
                })
            }
        }
    }
}

// 完整的声明式元素:头部 + 子节点。`element!` 用,`ToTokens` 委托 head 的 codegen。
pub struct ParsedElement {
    head: ParsedElementHead,
    children: Vec<ParsedElementChild>,
}

impl Parse for ParsedElement {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let head = input.parse::<ParsedElementHead>()?;
        let children = if input.peek(syn::token::Brace) {
            let children_input;
            braced!(children_input in input);
            parse_children(&children_input)?
        } else {
            Vec::new()
        };
        Ok(Self { head, children })
    }
}

impl ToTokens for ParsedElement {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend(self.head.to_element_expr(&self.children));
    }
}

pub enum ElementOrAdapter {
    Element(ParsedElement),
    // Adapter 装箱:ParsedAdapter 含两个 syn::Expr(stateful 的 widget+state),内联较大,
    // 不装箱会使本枚举因变体大小失衡触发 clippy::large_enum_variant。
    Adapter(Box<ParsedAdapter>),
}

impl Parse for ElementOrAdapter {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.peek(Ident) {
            let fork = input.fork();
            let ident: Ident = fork.parse()?;
            let ident = ident.to_string();
            if matches!(ident.as_str(), "widget" | "stateful") && fork.peek(syn::token::Paren) {
                let adapter: ParsedAdapter = input.parse()?;
                return Ok(ElementOrAdapter::Adapter(Box::new(adapter)));
            }
        }

        if input.peek(Token![$]) {
            return Err(input.error(
                "`$` adapter syntax was removed; use `widget(...)` or `stateful(widget, state)`",
            ));
        }

        if input.peek(Token![#]) {
            return Err(input.error("`#(expr)` child syntax was removed; use `{ expr }`"));
        }

        let element: ParsedElement = input.parse()?;
        Ok(ElementOrAdapter::Element(element))
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
