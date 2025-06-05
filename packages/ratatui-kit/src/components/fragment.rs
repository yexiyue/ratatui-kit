use ratatui_kit_macros::Props;

use crate::{AnyElement, Component, ComponentUpdater, Hooks};

#[derive(Default, Props)]
pub struct FragmentProps<'a> {
    /// The children of the component.
    pub children: Vec<AnyElement<'a>>,
}

#[derive(Default)]
pub struct Fragment;

impl Component for Fragment {
    type Props<'a> = FragmentProps<'a>;

    fn new(_props: &Self::Props<'_>) -> Self {
        Self
    }

    fn update(
        &mut self,
        props: &mut Self::Props<'_>,
        _hooks: Hooks,
        updater: &mut ComponentUpdater,
    ) {
        updater.set_transparent_layout(true);
        updater.update_children(props.children.iter_mut(), None);
    }
}
