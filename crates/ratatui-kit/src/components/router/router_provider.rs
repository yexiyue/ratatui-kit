// RouterProvider 组件：为终端应用提供路由上下文和历史管理，支持多页面、嵌套路由、参数等。
//
// 常与 Outlet、Routes 等配合，实现页面跳转和路由状态共享。
//
// ## 用法示例
// ```rust
// element!(RouterProvider(
//     routes: my_routes(),
//     index_path: "/".to_string(),
// ))
// ```
// 子组件可通过 hooks.use_navigate() 跳转页面，通过 hooks.use_route() 获取当前路由。

use crate::{
    Component, Context, Hooks, UseState,
    components::router::history::RouterHistory,
    prelude::{ContextProvider, Outlet, RouteContext, RouteState, Routes},
};
use ratatui_kit_macros::{Props, element};
use std::collections::HashMap;

#[derive(Default, Props)]
// RouterProvider 组件属性。
pub struct RouterProviderProps {
    // 路由表。
    pub routes: Routes,
    // 默认首页路径。
    pub index_path: String,
    // 路由历史最大长度。可直接传 `usize`(自动 `Some`)或 `Option<usize>`。
    pub history_length: Option<usize>,
    // 可选的路由状态。可直接传 `RouteState`(自动 `Some`)或 `Option<RouteState>`。
    pub state: Option<RouteState>,
}

pub struct RouterProvider;

impl Component for RouterProvider {
    type Props<'a> = RouterProviderProps;

    fn new(_props: &Self::Props<'_>) -> Self {
        Self
    }

    fn update(
        &mut self,
        props: &mut Self::Props<'_>,
        mut hooks: Hooks,
        updater: &mut crate::ComponentUpdater,
    ) {
        let history = hooks.use_state(|| {
            RouterHistory::new(
                RouteContext {
                    params: HashMap::new(),
                    path: props.index_path.clone(),
                    state: props.state.clone(),
                },
                props.history_length.unwrap_or(10),
            )
        });

        let ctx = history.read().current_context();

        updater.update_children(
            [element!(
                ContextProvider(
                    value: Context::owned(history),
                ) {
                    ContextProvider(
                        value: Context::owned(ctx),
                    ){
                        Outlet
                    }
                }
            )],
            Some(Context::from_mut(&mut props.routes)),
        );
    }
}
