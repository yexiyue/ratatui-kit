use crate::{AnyElement, Component, layout_style::LayoutStyle};
use ratatui::{
    layout::{Constraint, Rect},
    widgets::Clear,
};
use ratatui_kit_macros::Props;

#[derive(Default)]
pub struct Positioned {
    area: Rect,
    clear: bool,
}

#[derive(Default, Props)]
pub struct PositionedProps<'a> {
    /// 是否在渲染前清除该区域内容，默认为 false。
    pub clear: bool,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub children: Vec<AnyElement<'a>>,
}

impl Component for Positioned {
    type Props<'a> = PositionedProps<'a>;

    fn new(props: &Self::Props<'_>) -> Self {
        Self {
            area: Rect::new(props.x, props.y, props.width, props.height),
            clear: props.clear,
        }
    }

    fn update(
        &mut self,
        props: &mut Self::Props<'_>,
        _hooks: crate::Hooks,
        updater: &mut crate::ComponentUpdater,
    ) {
        *self = Self {
            area: Rect::new(props.x, props.y, props.width, props.height),
            clear: props.clear,
        };

        updater.update_children(&mut props.children, None);
        updater.set_layout_style(LayoutStyle {
            width: Constraint::Length(0),
            height: Constraint::Length(0),
            ..Default::default()
        });
    }

    fn draw(&mut self, drawer: &mut crate::ComponentDrawer<'_, '_>) {
        if self.clear {
            drawer.render_widget(Clear, self.area);
        }
        drawer.area = self.area;
    }
}
