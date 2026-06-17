// Outlet 组件：路由嵌套出口，根据当前路径动态渲染匹配的子路由组件。
//
// 通常与 RouterProvider、Routes 等配合使用，实现多级页面嵌套和动态参数解析。
//
// 类似于 React Router 的 <Outlet />，用于在父路由中渲染匹配的子路由内容，支持递归嵌套和参数传递。

use crate::{
    AnyElement, Component, ComponentUpdater, Context, Hooks, NoProps, UseContext, element,
    prelude::{ContextProvider, Fragment, Route, RouteContext, Routes},
};

// Outlet 组件实现。
pub struct Outlet;

impl Component for Outlet {
    type Props<'a> = NoProps;

    fn new(_props: &Self::Props<'_>) -> Self {
        Self
    }

    fn update(
        &mut self,
        _props: &mut Self::Props<'_>,
        mut hooks: Hooks,
        updater: &mut ComponentUpdater,
    ) {
        let Some((current_index, route_snapshot, mut component, mut children)) = ({
            let hooks = hooks.with_context_stack(updater.component_context_stack());
            let mut routes = hooks.use_context_mut::<Routes>();
            let mut route_context = hooks.use_context_mut::<RouteContext>();

            let path = route_context.path.clone();
            let mut current_index = None;
            for (index, route) in routes.iter_mut().enumerate() {
                if let Some((rest, params)) = route.match_path(&path) {
                    route_context.params.extend(params);
                    route_context.path = rest;
                    current_index = Some(index);
                    break;
                }
            }

            if current_index.is_none() {
                current_index = routes.iter().position(|route| route.path == "/");
            }

            current_index.map(|index| {
                let current_route = &mut routes[index];
                let route_snapshot = Route {
                    path: current_route.path.clone(),
                    component: element!(Fragment).into_any(),
                    children: Routes::default(),
                    matcher: current_route.matcher.clone(),
                };
                let component =
                    std::mem::replace(&mut current_route.component, element!(Fragment).into_any());
                let children = std::mem::take(&mut current_route.children);
                (index, route_snapshot, component, children)
            })
        }) else {
            #[cfg(debug_assertions)]
            eprintln!("ratatui-kit router: no route matched current path");
            updater.update_children(Vec::<AnyElement<'_>>::new(), None);
            return;
        };

        updater.update_children(
            [
                element!(ContextProvider(value: Context::owned(route_snapshot)) {
                    { &mut component }
                }),
            ],
            Some(Context::from_mut(&mut children)),
        );

        let hooks = hooks.with_context_stack(updater.component_context_stack());
        let mut routes = hooks.use_context_mut::<Routes>();
        routes[current_index].component = component;
        routes[current_index].children = children;
    }
}
