use crate::{AnyElement, Component, Context};
use ratatui_kit_macros::Props;

#[derive(Default, Props)]
pub struct ContextProviderProps<'a> {
    pub children: Vec<AnyElement<'a>>,
    pub value: Option<Context<'a>>,
}

pub struct ContextProvider;

impl Component for ContextProvider {
    type Props<'a> = ContextProviderProps<'a>;
    fn new(_props: &Self::Props<'_>) -> Self {
        Self
    }

    fn update(
        &mut self,
        props: &mut Self::Props<'_>,
        _hooks: crate::Hooks,
        updater: &mut crate::ComponentUpdater,
    ) {
        updater.set_transparent_layout(true);
        updater.update_children(
            props.children.iter_mut(),
            props.value.as_mut().map(|v| v.borrow()),
        );
    }
}
