use std::ops::{Deref, DerefMut};

use ratatui::{style::Style, widgets::Paragraph};
use ratatui_kit_macros::{Props, component};

use crate::{AnyElement, element, prelude::Fragment};

#[derive(Clone, Default)]
pub struct TextParagraph<'a> {
    inner: Paragraph<'a>,
}

impl<'a> Deref for TextParagraph<'a> {
    type Target = Paragraph<'a>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for TextParagraph<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl From<String> for TextParagraph<'_> {
    fn from(value: String) -> Self {
        Self {
            inner: Paragraph::new(value),
        }
    }
}

impl<'a> From<Paragraph<'a>> for TextParagraph<'a> {
    fn from(value: Paragraph<'a>) -> Self {
        Self { inner: value }
    }
}

#[derive(Default, Props)]
pub struct TextProps {
    pub text: TextParagraph<'static>,
    pub style: Style,
    pub alignment: ratatui::layout::Alignment,
    pub wrap: bool,
}

#[component]
pub fn Text(props: &TextProps) -> impl Into<AnyElement<'static>> {
    let paragraph = props
        .text
        .inner
        .clone()
        .style(props.style)
        .alignment(props.alignment)
        .wrap(if props.wrap {
            ratatui::widgets::Wrap { trim: true }
        } else {
            ratatui::widgets::Wrap { trim: false }
        });

    element! {
        Fragment{
            $paragraph
        }
    }
}
