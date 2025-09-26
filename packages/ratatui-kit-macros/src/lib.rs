//! ratatui-kit-macros：核心 UI 宏定义，简化终端 UI 组件开发。
//!
//! ## 主要宏说明
//!
//! - `#[derive(Props)]`：为组件属性自动生成 Props trait 实现。
//! - `element!`：声明式 UI 宏，极大提升终端 UI 组件开发效率。
//!   - 语法风格类似 React JSX，但为 Rust 语法友好设计。
//!   - 支持嵌套、props、children、条件渲染、列表渲染。
//!   - 条件渲染、列表渲染、动态子组件等均需写在 `#(...expr)` 语法块中，表达式可返回 Option/Vec/impl Iterator。
//!   - 通过 `$` 前缀可兼容任何实现 WidgetRef 的 ratatui 原生组件或自定义组件，便于无缝集成 ratatui 能力。
//!   - 适用于声明式构建终端 UI 组件树。
//!
//! ## element! 宏语法
//!
//! 例如，声明式构建一个带条件渲染和 ratatui 原生组件的 UI：
//!
//! ```rust
//! element!(Panel(title: "Demo") {
//!     #(if show_title { element!(Title("Hello")) }),
//!     #(for item in items { element!(ListItem(item)) }),
//!     $Block::default().borders(Borders::ALL),
//! })
//! ```
//!
//! - 所有条件渲染、列表渲染、动态子组件都需包裹在 `#(...)` 表达式中，且条件渲染/循环渲染的子组件也需用 element! 宏包裹。
//! - 通过 `$` 前缀可直接集成 ratatui 原生组件。
//! - 语法风格类似 JSX，但为 Rust 语法友好设计。
//! - 适用于声明式构建终端 UI 组件树。

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
#[cfg(feature = "store")]
mod store;
mod utils;
mod with_layout_style;

#[proc_macro_derive(Props, attributes(layout))]
pub fn derive_props(item: TokenStream) -> TokenStream {
    let props = syn::parse_macro_input!(item as ParsedProps);
    props.to_token_stream().into()
}

/// 声明式 UI 宏，类似 JSX，支持嵌套、props、children、条件渲染、列表渲染等，极大提升终端 UI 组件开发效率。
///
/// - 语法风格类似 React JSX，但为 Rust 语法友好设计。
/// - 支持 `if/else` 条件渲染、`#(for ...)` 列表渲染、props 传递、children 嵌套。
/// - 通过 `$` 前缀可兼容任何实现 WidgetRef 的 ratatui 原生组件或自定义组件，便于无缝集成 ratatui 能力。
/// - 适用于声明式构建终端 UI 组件树。
///
/// ## element! 宏语法
///
/// 例如，声明式构建一个带条件渲染和 ratatui 原生组件的 UI：
///
/// ```rust
/// element!(Panel(title: "Demo") {
///     #(if show_title { element!(Title("Hello")) }),
///     #(for item in items { element!(ListItem(item)) }),
///     $Block::default().borders(Borders::ALL),
/// })
/// ```
#[proc_macro]
pub fn element(input: TokenStream) -> TokenStream {
    let element = syn::parse_macro_input!(input as ElementOrAdapter);
    element.to_token_stream().into()
}

/// 简化组件函数定义，自动实现 Component trait。
#[proc_macro_attribute]
pub fn component(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let component = syn::parse_macro_input!(item as component::ParsedComponent);
    component.to_token_stream().into()
}

#[cfg(feature = "router")]
#[proc_macro]
pub fn routes(input: TokenStream) -> TokenStream {
    let routes = syn::parse_macro_input!(input as router::Routes);
    routes.to_token_stream().into()
}

#[cfg(feature = "store")]
#[proc_macro]
pub fn use_stores(input: TokenStream) -> TokenStream {
    let stores = syn::parse_macro_input!(input as store::UseStores);
    stores.to_token_stream().into()
}

#[cfg(feature = "store")]
#[proc_macro_derive(Store)]
pub fn derive_store(item: TokenStream) -> TokenStream {
    let store = syn::parse_macro_input!(item as store::Store);
    store.to_token_stream().into()
}

/// 为属性结构体自动生成布局相关方法。
#[proc_macro_attribute]
pub fn with_layout_style(attr: TokenStream, item: TokenStream) -> TokenStream {
    let layout_style = syn::parse_macro_input!(attr as with_layout_style::ParsedLayoutStyle);
    let props = syn::parse_macro_input!(item as DeriveInput);
    impl_layout_style(&layout_style, props).into()
}
