use proc_macro2::Span;
use quote::{ToTokens, quote};
use syn::{
    Expr, FieldValue, Member, Pat, Token, TypePath, braced, parse::Parse, parse::ParseStream,
    punctuated::Punctuated, spanned::Spanned, token::Comma,
};
use uuid::Uuid;

use crate::adapter::ParsedAdapter;

/// 单个子节点：嵌套元素 / `$` 适配器、`#(expr)` 任意表达式、或一等控制流(if/for/match)。
enum ParsedElementChild {
    Element(ElementOrAdapter),
    Expr(Expr),
    ControlFlow(ControlFlow),
}

/// element! 子节点块内的一等控制流。分支体本身又是一组子节点。
///
/// 相比 `#(if cond { elem.into_any() } else { ... })`,一等控制流让每个分支独立把自己的
/// 子节点 `extend` 进 children——故各分支可返回不同元素类型,无需 `.into_any()` 统一类型。
enum ControlFlow {
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

/// `else if ...` 或 `else { ... }`。
enum ElseBranch {
    If(Box<ControlFlow>),
    Block(Vec<ParsedElementChild>),
}

struct MatchArm {
    pat: Pat,
    guard: Option<Expr>,
    body: Vec<ParsedElementChild>,
}

/// 解析一段子节点序列(用于元素的 `{}` 块,以及控制流的各分支体)。
fn parse_children(input: ParseStream) -> syn::Result<Vec<ParsedElementChild>> {
    let mut children = Vec::new();
    while !input.is_empty() {
        if input.peek(Token![#]) {
            // `#(expr)`:把子节点位置交还给任意 Rust 表达式(返回 Option/Vec/Iterator/Element)。
            input.parse::<Token![#]>()?;
            let expr;
            syn::parenthesized!(expr in input);
            children.push(ParsedElementChild::Expr(expr.parse()?));
        } else if input.peek(Token![if]) {
            children.push(ParsedElementChild::ControlFlow(parse_if(input)?));
        } else if input.peek(Token![for]) {
            children.push(ParsedElementChild::ControlFlow(parse_for(input)?));
        } else if input.peek(Token![match]) {
            children.push(ParsedElementChild::ControlFlow(parse_match(input)?));
        } else {
            // 嵌套元素 `Comp(..){..}` 或 `$widget` 适配器。
            children.push(ParsedElementChild::Element(input.parse()?));
        }
    }
    Ok(children)
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
    /// 生成「把本子节点 extend 进 `dest`」的语句。控制流会把内层 extend 包进 if/for/match。
    fn to_extend(&self, dest: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        // Element 与 Expr 都走同一条 extend_with_elements 通道,只是内层 token 不同;
        // ControlFlow 则递归把内层 extend 包进 if/for/match 外壳。
        let child = match self {
            ParsedElementChild::Element(element) => quote!(#element),
            ParsedElementChild::Expr(expr) => quote!(#expr),
            ParsedElementChild::ControlFlow(cf) => return cf.to_extend(dest),
        };
        quote!(::ratatui_kit::extend_with_elements(&mut #dest, #child);)
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
}

pub struct ParsedElement {
    ty: TypePath,
    props: Punctuated<PropsItem, Comma>,
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

        let children = if input.peek(syn::token::Brace) {
            let children_input;
            braced!(children_input in input);
            parse_children(&children_input)?
        } else {
            Vec::new()
        };

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
        let mut has_rest = false;
        // 有用户 `key:` → User((decl_key, expr)) 单次堆分配;否则 Decl(decl_key) 零堆分配。
        let key = self
            .props
            .iter()
            .find_map(|props_item: &PropsItem| match props_item {
                PropsItem::FieldValue(FieldValue { member, expr, .. }) => match member {
                    Member::Named(ident) if ident == "key" => {
                        Some(quote!(::ratatui_kit::ElementKey::user((#decl_key, #expr))))
                    }
                    _ => None,
                },
                PropsItem::Rest(_) => {
                    has_rest = true;
                    None
                }
            })
            .unwrap_or_else(|| quote!(::ratatui_kit::ElementKey::decl(#decl_key)));

        let props_assignments = self
            .props
            .iter()
            .filter_map(|props_item: &PropsItem| match props_item {
                PropsItem::FieldValue(FieldValue { member, .. }) => match member {
                    Member::Named(ident) if ident == "key" => None,
                    _ => Some(quote!(#props_item)),
                },
                _ => Some(quote!(#props_item)),
            })
            .collect::<Vec<_>>();

        let set_children = if !self.children.is_empty() {
            let dest = quote!(_element.props.children);
            let stmts = self.children.iter().map(|child| child.to_extend(&dest));
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

        if has_props_assignments {
            tokens.extend(quote! {
                {
                    type Props<'a>= <#ty as ::ratatui_kit::ElementType>::Props<'a>;
                    let mut _props = Props{
                        #default_rest
                    };

                    let mut _element=::ratatui_kit::Element::<#ty>{
                        key: #key,
                        props: _props,
                    };
                    #set_children
                    _element
                }
            });
        } else {
            tokens.extend(quote! {
                {
                    type Props<'a>= <#ty as ::ratatui_kit::ElementType>::Props<'a>;
                    let mut _props = Props::default();
                    let mut _element=::ratatui_kit::Element::<#ty>{
                        key: #key,
                        props: _props,
                    };
                    #set_children
                    _element
                }
            });
        }
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
