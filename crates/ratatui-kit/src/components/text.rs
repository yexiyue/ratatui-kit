use crate::{
    AnyElement, ComponentTheme, Hooks, Palette, UseTheme, components::theme::resolve_style,
    element, prelude::Fragment,
};
use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect},
    style::Style,
    text::{Line, Text as RataText},
    widgets::{Paragraph, Widget},
};
use ratatui_kit_macros::{Props, component};
use std::ops::{Deref, DerefMut};

/// Text 组件的主题 slot。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextTheme {
    /// 正文文本样式。
    pub style: Style,
}

impl ComponentTheme for TextTheme {
    fn from_palette(palette: &Palette) -> Self {
        Self {
            style: Style::new().fg(palette.fg),
        }
    }
}

impl Default for TextTheme {
    fn default() -> Self {
        Self::from_palette(&Palette::default())
    }
}

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

// 让 `TextParagraph` 成为**按值** `Widget`,以匹配 WidgetAdapter 改后的 `T: Widget` 约束。
// `Paragraph` 0.30 起本就是按值 Widget,直接消费式转发。
impl Widget for TextParagraph<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.inner.render(area, buf);
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

// 让 Text 组件的 `text:` 字段直接吃字符串字面量 / Line / Text(都经 `(#expr).into()`),
// 从而 `Text(text: "速度:", style: s)` 可替代高频的 `$Line::from("速度:").style(s)`。
impl<'a> From<&'a str> for TextParagraph<'a> {
    fn from(value: &'a str) -> Self {
        Self {
            inner: Paragraph::new(value),
        }
    }
}

impl<'a> From<Line<'a>> for TextParagraph<'a> {
    fn from(value: Line<'a>) -> Self {
        Self {
            inner: Paragraph::new(value),
        }
    }
}

impl<'a> From<RataText<'a>> for TextParagraph<'a> {
    fn from(value: RataText<'a>) -> Self {
        Self {
            inner: Paragraph::new(value),
        }
    }
}

#[derive(Default, Props)]
pub struct TextProps {
    pub text: TextParagraph<'static>,
    // 文本样式覆盖。`None` 用主题(`TextTheme`,从 `Palette` 派生),`Some(s)` 以 `theme.patch(s)` 覆盖。
    pub style: Option<Style>,
    pub alignment: ratatui::layout::Alignment,
    pub scroll: Position,
    // 是否换行(trim)。可直接传 `bool`(自动 `Some`)或 `Option<bool>`。
    pub wrap: Option<bool>,
}

#[component]
pub fn Text(props: &TextProps, hooks: Hooks) -> impl Into<AnyElement<'static>> {
    // 主题解析:theme slot 铺底,props 的 Option<Style> 在上 patch(None → 用主题)。
    let theme = hooks.use_component_theme::<TextTheme>();
    let style = resolve_style(theme.style, props.style);

    let paragraph = props
        .text
        .inner
        .clone()
        .style(style)
        .scroll((props.scroll.x, props.scroll.y))
        .alignment(props.alignment);

    let paragraph = if let Some(wrap) = props.wrap {
        paragraph.wrap(ratatui::widgets::Wrap { trim: wrap })
    } else {
        paragraph
    };

    let paragraph = TextParagraph::from(paragraph);

    element! {
        Fragment{
            widget(paragraph)
        }
    }
}
