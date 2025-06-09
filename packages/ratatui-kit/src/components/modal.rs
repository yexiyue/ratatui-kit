use ratatui::{
    layout::{Constraint, Flex, Layout, Margin, Offset},
    style::Style,
    widgets::{Block, Clear},
};
use ratatui_kit_macros::Props;

use crate::{AnyElement, Component, layout_style::LayoutStyle};

#[derive(Default, Clone, Copy)]
pub enum Placement {
    Top,
    TopLeft,
    TopRight,
    Bottom,
    BottomLeft,
    BottomRight,
    #[default]
    Center,
    Left,
    Right,
}

impl Placement {
    pub fn to_flex(&self) -> [Flex; 2] {
        match self {
            Placement::Top => [Flex::Start, Flex::Center],
            Placement::TopLeft => [Flex::Start, Flex::Start],
            Placement::TopRight => [Flex::Start, Flex::End],
            Placement::Bottom => [Flex::End, Flex::Center],
            Placement::BottomLeft => [Flex::End, Flex::Start],
            Placement::BottomRight => [Flex::End, Flex::End],
            Placement::Center => [Flex::Center, Flex::Center],
            Placement::Left => [Flex::Center, Flex::Start],
            Placement::Right => [Flex::Center, Flex::End],
        }
    }
}

#[derive(Default, Props)]
pub struct ModalProps<'a> {
    pub children: Vec<AnyElement<'a>>,
    pub margin: Margin,
    pub offset: Offset,
    pub width: Constraint,
    pub height: Constraint,
    pub style: Style,
    pub placement: Placement,
    pub open: bool,
}

pub struct Modal {
    pub open: bool,
    pub margin: Margin,
    pub offset: Offset,
    pub width: Constraint,
    pub height: Constraint,
    pub placement: Placement,
    pub style: Style,
}

impl Component for Modal {
    type Props<'a> = ModalProps<'a>;
    fn new(props: &Self::Props<'_>) -> Self {
        Modal {
            open: props.open,
            margin: props.margin,
            offset: props.offset,
            width: props.width,
            height: props.height,
            style: props.style,
            placement: props.placement,
        }
    }

    fn update(
        &mut self,
        props: &mut Self::Props<'_>,
        _hooks: crate::Hooks,
        updater: &mut crate::ComponentUpdater,
    ) {
        self.open = props.open;
        self.margin = props.margin;
        self.offset = props.offset;
        self.width = props.width;
        self.height = props.height;
        self.style = props.style;
        self.placement = props.placement;

        if self.open {
            updater.update_children(props.children.iter_mut(), None);
        }

        updater.set_layout_style(LayoutStyle {
            width: Constraint::Percentage(0),
            height: Constraint::Percentage(0),
            ..Default::default()
        });
    }

    fn draw(&mut self, drawer: &mut crate::ComponentDrawer<'_, '_>) {
        if self.open {
            let area = drawer.frame.area();
            let area = area.inner(self.margin).offset(self.offset);
            let block = Block::default().style(self.style);
            drawer.frame.render_widget(block, area);

            let [v, h] = self.placement.to_flex();

            let vertical = Layout::vertical([self.height]).flex(v).split(area)[0];
            let horizontal = Layout::horizontal([self.width]).flex(h).split(vertical)[0];

            drawer.frame.render_widget(Clear, horizontal);
            drawer.area = horizontal;
        }
    }
}
