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

pub(crate) fn split_path(path: &str) -> VecDeque<String> {
    let mut res = VecDeque::new();
    let reg = regex::Regex::new(r"(\/[^/]+)").unwrap();

    for cap in reg.captures_iter(path) {
        res.push_back(cap[1].to_string());
    }

    res
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
            path: split_path(&props.index_path),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_path() {
        let path = "/a/b/c";
        let result = split_path(path);
        assert_eq!(
            result,
            VecDeque::from(vec!["/a".to_string(), "/b".to_string(), "/c".to_string()])
        );
    }
}
