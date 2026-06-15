//! Outlet 组件：路由嵌套出口，根据当前路径动态渲染匹配的子路由组件。
//!
//! 通常与 RouterProvider、Routes 等配合使用，实现多级页面嵌套和动态参数解析。
//!
//! 类似于 React Router 的 <Outlet />，用于在父路由中渲染匹配的子路由内容，支持递归嵌套和参数传递。

use crate::{
    AnyElement, Context, Hooks, UseContext,
    prelude::{ContextProvider, RouteContext, Routes},
};
use ratatui_kit_macros::{component, element};

/// Outlet 组件实现。
#[component]
pub fn Outlet<'a>(hooks: Hooks) -> impl Into<AnyElement<'a>> {
    // 获取全局路由表和当前路径上下文
    let mut routes = hooks.use_context_mut::<Routes>();
    let mut route_context = hooks.use_context_mut::<RouteContext>();

    // 查找与当前路径匹配的第一个路由（匹配逻辑见 Route::match_path）。
    // 命中则把提取的参数并入上下文、并把路径推进为剩余未匹配部分,供嵌套 Outlet 续匹配。
    let mut current_route = routes.iter_mut().find(|r| {
        let path = route_context.path.clone();
        match r.match_path(&path) {
            Some((rest, params)) => {
                route_context.params.extend(params);
                route_context.path = rest;
                true
            }
            None => false,
        }
    });

    // 如果没有找到匹配的路由，则尝试匹配根路径 "/"
    if current_route.is_none() {
        current_route = routes.iter_mut().find(|r| r.path == "/");
    }

    // 解包 Option 并确保存在匹配的路由
    let current_route = current_route.expect("No matching route found");

    // 构建当前路由对应的 UI 元素
    let current_element = AnyElement::from(&mut current_route.component);

    // 返回构建的 UI 树结构
    element!(ContextProvider(
        value: Context::owned(current_route.children.borrow())
    ) {
        ContextProvider(
            value: Context::owned(current_route.borrow())
        ) {
            { current_element }
        }
    })
}
