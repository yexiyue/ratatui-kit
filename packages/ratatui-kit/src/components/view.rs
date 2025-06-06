use ratatui::layout::{Constraint, Direction, Flex, Margin, Offset};
use ratatui_kit_macros::Props;

use crate::{AnyElement, Component, layout_style::LayoutStyle};

#[derive(Default, Props)]
pub struct ViewProps<'a> {
    pub flex_direction: Direction,
    pub justify_content: Flex,
    pub gap: i32,
    pub margin: Margin,
    pub offset: Offset,
    pub width: Constraint,
    pub height: Constraint,
    pub children: Vec<AnyElement<'a>>,
}

pub struct View;

impl<'a> From<&ViewProps<'a>> for LayoutStyle {
    fn from(props: &ViewProps) -> Self {
        LayoutStyle {
            flex_direction: props.flex_direction,
            justify_content: props.justify_content,
            gap: props.gap,
            margin: props.margin,
            offset: props.offset,
            width: props.width,
            height: props.height,
        }
    }
}

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
        let layout_style = LayoutStyle::from(&*props);
        updater.set_layout_style(layout_style);
        updater.update_children(&mut props.children, None);
    }
}
