use ratatui::{layout::Alignment, widgets::Widget};
use ratatui_kit_macros::Props;

use crate::Component;

#[derive(Debug, Default, Props, Clone)]
pub struct TextProps {
    pub text: String,
    pub alignment: Option<Alignment>,
    pub style: ratatui::style::Style,
}

pub struct Text {
    props: TextProps,
}

impl Component for Text {
    type Props<'a> = TextProps;

    fn new(props: &TextProps) -> Self {
        Self {
            props: props.clone(),
        }
    }

    fn update(
        &mut self,
        props: &mut Self::Props<'_>,
        _hooks: crate::Hooks,
        _updater: &mut crate::ComponentUpdater,
    ) {
        self.props = props.clone();
    }

    fn render_ref(&self, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer) {
        let text_widget = ratatui::text::Text::styled(&self.props.text, self.props.style)
            .alignment(self.props.alignment.unwrap_or_default());
        text_widget.render(area, buf);
    }
}
