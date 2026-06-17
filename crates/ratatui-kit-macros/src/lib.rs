#![doc = include_str!("../README.md")]

// ratatui-kit-macros：核心 UI 宏定义，简化终端 UI 组件开发。
//
// ## 主要宏说明
//
// - `#[derive(Props)]`：为组件属性自动生成 Props trait 实现。
// - `element!`：声明式 UI 宏，极大提升终端 UI 组件开发效率。
//   - 语法风格类似 React JSX，但为 Rust 语法友好设计。
//   - 支持嵌套、props、children、一等控制流渲染。
//   - **一等控制流**：子节点块内可直接写 `if/else`、`if let`、`for`、`match`，分支体即子节点；
//     各分支独立 extend，可返回不同元素类型，无需 `.into_any()`。
//   - `{ expr }` 可内嵌任意返回 Option/Vec/impl Iterator/Element 的 Rust 表达式。
//   - `widget(expr)` / `stateful(widget, state)` 可兼容 ratatui 原生组件（逃生舱）。
//   - 适用于声明式构建终端 UI 组件树。
//
// ## element! 宏语法
//
// 例如，声明式构建一个带一等控制流和 ratatui 原生组件的 UI：
//
// ```rust,ignore
// element!(Panel(title: "Demo") {
//     if show_title {
//         Title(text: "Hello")
//     }
//     for item in items {
//         ListItem(label: item, key: item.id)
//     }
//     widget(Block::default().borders(Borders::ALL))
// })
// ```
//
// - 控制流分支体直接写子元素；动态/复杂表达式仍可用 `{ expr }`。
// - 通过 `widget(...)` / `stateful(...)` 可直接集成 ratatui 原生组件。
// - 语法风格类似 JSX，但为 Rust 语法友好设计。
// - 适用于声明式构建终端 UI 组件树。

use element::ElementOrAdapter;
use proc_macro::TokenStream;
use props::ParsedProps;
use quote::ToTokens;
use syn::DeriveInput;

use crate::with_layout_style::impl_layout_style;

mod adapter;
mod component;
mod element;
mod props;
#[cfg(feature = "router")]
mod router;
mod utils;
mod with_layout_style;

/// Derives Ratatui Kit's `Props` implementation for component props.
#[proc_macro_derive(Props, attributes(layout))]
pub fn derive_props(item: TokenStream) -> TokenStream {
    let props = syn::parse_macro_input!(item as ParsedProps);
    props.to_token_stream().into()
}

/// Builds a declarative Ratatui Kit element tree.
///
/// The syntax is JSX-like while staying Rust-friendly: nested components,
/// props, children, `key`, first-class `if`/`if let`/`for`/`match` child
/// control flow, and native Ratatui widget adapters are supported.
///
/// ```rust,ignore
/// element!(Panel(title: "Demo") {
///     if show_title {
///         Title(text: "Hello")
///     } else {
///         Title(text: "Hidden")
///     }
///     for item in items {
///         ListItem(label: item, key: item.id)
///     }
///     widget(Block::default().borders(Borders::ALL))
/// })
/// ```
#[proc_macro]
pub fn element(input: TokenStream) -> TokenStream {
    let element = syn::parse_macro_input!(input as ElementOrAdapter);
    element.to_token_stream().into()
}

/// Turns a function into a Ratatui Kit component.
///
/// The generated component owns the props type, preserves hook order, and
/// enables context-aware hooks for ordinary function components.
#[proc_macro_attribute]
pub fn component(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let component = syn::parse_macro_input!(item as component::ParsedComponent);
    component.to_token_stream().into()
}

/// Builds a static route table for `RouterProvider`.
///
/// Routes are matched in declaration order. Put more specific static routes
/// before same-prefix dynamic routes when both could match the same path.
#[cfg(feature = "router")]
#[proc_macro]
pub fn routes(input: TokenStream) -> TokenStream {
    let routes = syn::parse_macro_input!(input as router::Routes);
    routes.to_token_stream().into()
}

/// Adds layout style fields and helpers to a props struct.
///
/// Use this on named-field props structs for components that should accept
/// layout props such as width, height, margin, offset, gap, direction, and
/// justification.
#[proc_macro_attribute]
pub fn with_layout_style(attr: TokenStream, item: TokenStream) -> TokenStream {
    let layout_style = syn::parse_macro_input!(attr as with_layout_style::ParsedLayoutStyle);
    let props = syn::parse_macro_input!(item as DeriveInput);
    impl_layout_style(&layout_style, props).into()
}
