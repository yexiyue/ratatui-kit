use crate::{
    AnyElement, Context, Hooks, UseState,
    components::router::history::RouterHistory,
    prelude::{ContextProvider, Outlet, RouteContext, Routes},
};
use ratatui_kit_macros::{Props, component, element};
use std::collections::{HashMap, VecDeque};

#[derive(Default, Props)]
pub struct RouterProviderProps {
    pub routes: Routes,
    pub index_path: String,
    pub history_length: Option<usize>,
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
            state: None,
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
