//! RouterProvider 组件：为终端应用提供路由上下文和历史管理，支持多页面、嵌套路由、参数等。
//!
//! 常与 Outlet、Routes 等配合，实现页面跳转和路由状态共享。
//!
//! ## 用法示例
//! ```rust
//! element!(RouterProvider(
//!     routes: my_routes(),
//!     index_path: "/".to_string(),
//! ))
//! ```
//! 子组件可通过 hooks.use_navigate() 跳转页面，通过 hooks.use_route() 获取当前路由。

use crate::{
    AnyElement, Context, Hooks, UseState,
    components::router::history::RouterHistory,
    prelude::{ContextProvider, Outlet, RouteContext, RouteState, Routes},
};
use ratatui_kit_macros::{Props, component, element};
use std::collections::{HashMap, VecDeque};

#[derive(Default, Props)]
/// RouterProvider 组件属性。
pub struct RouterProviderProps {
    /// 路由表。
    pub routes: Routes,
    /// 默认首页路径。
    pub index_path: String,
    /// 路由历史最大长度。
    pub history_length: Option<usize>,
    /// 可选的路由状态。
    pub state: Option<RouteState>,
}

#[component]
pub fn RouterProvider<'a>(
    props: &mut RouterProviderProps,
    mut hooks: Hooks,
) -> impl Into<AnyElement<'a>> {
    let history = hooks.use_state(|| RouterHistory {
        current: 0,
        max_length: props.history_length.unwrap_or(10),
        history: VecDeque::from(vec![RouteContext {
            params: HashMap::new(),
            path: props.index_path.clone(),
            state: props.state.clone(),
        }]),
    });

    let ctx = history.read().current_context();

    element!(
        ContextProvider(
            value: Context::owned(history),
        ) {
            ContextProvider(
                value: Context::owned(ctx),
            ){
                ContextProvider(
                    value: Context::owned(props.routes.borrow()),
                ) {
                    Outlet
                }
            }
        }
    )
}
