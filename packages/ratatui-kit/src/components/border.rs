use ratatui::{
    symbols::border,
    text::Line,
    widgets::{Block, Padding, Widget},
};
use ratatui_kit_macros::{Props, with_layout_style};

use crate::{AnyElement, Component};

#[with_layout_style]
#[derive(Props)]
pub struct BorderProps<'a> {
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
            padding: Padding::default(),
            border_style: ratatui::style::Style::default(),
            borders: ratatui::widgets::Borders::ALL,
            children: Vec::new(),
            border_set: border::Set::default(),
            style: ratatui::style::Style::default(),
            top_title: None,
            bottom_title: None,
            margin: Default::default(),
            offset: Default::default(),
            width: Default::default(),
            height: Default::default(),
            gap: Default::default(),
            flex_direction: Default::default(),
            justify_content: Default::default(),
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
        let layout_style = props.layout_style();
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
        block.render(drawer.area, drawer.buffer_mut());
        drawer.area = inner_area;
    }
}
