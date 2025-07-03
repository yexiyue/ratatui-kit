use ratatui_kit_macros::{Props, with_layout_style};

use crate::{AnyElement, Component};

#[with_layout_style]
#[derive(Default, Props)]
pub struct ViewProps<'a> {
    pub children: Vec<AnyElement<'a>>,
}

pub struct View;

impl Component for View {
    type Props<'a> = ViewProps<'a>;

    fn new(_props: &Self::Props<'_>) -> Self {
        Self
    }

    fn update(
        &mut self,
        props: &mut Self::Props<'_>,
        _hooks: crate::Hooks,
        updater: &mut crate::ComponentUpdater,
    ) {
        updater.set_layout_style(props.layout_style());
        updater.update_children(&mut props.children, None);
    }
}
