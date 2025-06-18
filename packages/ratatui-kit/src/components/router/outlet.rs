use crate::{
    AnyElement, Context, Hooks, UseContext,
    prelude::{ContextProvider, RouteContext, Routes},
};
use ratatui_kit_macros::{component, element};

#[component]
pub fn Outlet<'a>(hooks: Hooks) -> impl Into<AnyElement<'a>> {
    let mut routes = hooks.use_context_mut::<Routes>();
    let mut route_context = hooks.use_context_mut::<RouteContext>();

    let current_route = routes
        .iter_mut()
        .find(|r| {
            let path = route_context.path.front().cloned().unwrap_or_default();
            let res = r.path == path;
            if res {
                route_context.path.pop_front();
            }
            res
        })
        .expect("current route not found");

    let current_element = AnyElement::from(&mut current_route.component);

    element!(ContextProvider(
        value:Context::owned(current_route.children.borrow()),
        ..Default::default()
    ){
        #(current_element)
    })
}
