use ratatui::{
    layout::{Constraint, Offset},
    symbols::border,
    text::Line,
    widgets::{Block, Padding},
};
use ratatui_kit_macros::Props;

use crate::{AnyElement, Component, layout_style::LayoutStyle};

#[derive(Props)]
pub struct BorderProps<'a> {
    pub offset: Offset,
    pub width: Constraint,
    pub height: Constraint,
    pub padding: Padding,
    pub border_style: ratatui::style::Style,
    pub borders: ratatui::widgets::Borders,
    pub border_set: border::Set,
    pub style: ratatui::style::Style,
    pub children: Vec<AnyElement<'a>>,
    pub top_title: Option<Line<'static>>,
    pub bottom_title: Option<Line<'static>>,
}

impl Default for BorderProps<'_> {
    fn default() -> Self {
        Self {
            offset: Offset::default(),
            width: Constraint::default(),
            height: Constraint::default(),
            padding: Padding::default(),
            border_style: ratatui::style::Style::default(),
            borders: ratatui::widgets::Borders::ALL,
            children: Vec::new(),
            border_set: border::Set::default(),
            style: ratatui::style::Style::default(),
            top_title: None,
            bottom_title: None,
        }
    }
}

impl From<&BorderProps<'_>> for LayoutStyle {
    fn from(props: &BorderProps) -> Self {
        LayoutStyle {
            offset: props.offset,
            width: props.width,
            height: props.height,
            ..Default::default()
        }
    }
}

pub struct Border {
    pub padding: Padding,
    pub border_style: ratatui::style::Style,
    pub borders: ratatui::widgets::Borders,
    pub border_set: border::Set,
    pub style: ratatui::style::Style,
    pub top_title: Option<Line<'static>>,
    pub bottom_title: Option<Line<'static>>,
}

impl Component for Border {
    type Props<'a> = BorderProps<'a>;

    fn new(props: &Self::Props<'_>) -> Self {
        Self {
            padding: props.padding,
            border_style: props.border_style,
            borders: props.borders,
            border_set: props.border_set,
            style: props.style,
            top_title: props.top_title.clone(),
            bottom_title: props.bottom_title.clone(),
        }
    }

    fn update(
        &mut self,
        props: &mut Self::Props<'_>,
        _hooks: crate::Hooks,
        updater: &mut crate::ComponentUpdater,
    ) {
        let layout_style = LayoutStyle::from(&*props);
        *self = Self {
            padding: props.padding,
            border_style: props.border_style,
            borders: props.borders,
            border_set: props.border_set,
            style: props.style,
            top_title: props.top_title.clone(),
            bottom_title: props.bottom_title.clone(),
        };
        updater.set_layout_style(layout_style);
        updater.update_children(&mut props.children, None);
    }

    fn draw(&mut self, drawer: &mut crate::ComponentDrawer<'_, '_>) {
        let mut block = Block::new()
            .style(self.style)
            .borders(self.borders)
            .border_set(self.border_set)
            .border_style(self.border_style)
            .padding(self.padding);

        if let Some(top_title) = &self.top_title {
            block = block.title_top(top_title.clone());
        }

        if let Some(bottom_title) = &self.bottom_title {
            block = block.title_bottom(bottom_title.clone());
        }

        let inner_area = block.inner(drawer.area);
        drawer.frame.render_widget(block, drawer.area);

        drawer.area = inner_area;
    }
}
