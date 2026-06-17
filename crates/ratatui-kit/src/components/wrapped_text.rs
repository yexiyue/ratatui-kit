// WrappedText 组件：按指定宽度自动换行，并把换行后的行数暴露给布局系统。
//
// 与 `Text(wrap: true)` 不同，`WrappedText` 会根据 `wrap_width` 计算自身高度，
// 因而适合放进 `ScrollView` 渲染长文档、日志、说明文本或小说正文。

use crate::{Component, ComponentDrawer, ComponentUpdater, Hooks};
use ratatui::{
    layout::{Alignment, Constraint, Position},
    style::Style,
    widgets::Paragraph,
};
use ratatui_kit_macros::{Props, with_layout_style};

const DEFAULT_WRAP_WIDTH: u16 = 80;

#[with_layout_style]
#[derive(Default, Props)]
// 自动换行文本属性。
pub struct WrappedTextProps {
    // 要渲染的纯文本。可直接传 `&str` 或 `String`。
    pub text: String,
    // 应用于整段文本的样式。
    pub style: Style,
    // 文本对齐方式。
    pub alignment: Alignment,
    // 文本滚动偏移，语义与 ratatui `Paragraph::scroll` 保持一致。
    pub scroll: Position,
    // 用于计算换行和自动高度的宽度。长正文放进 `ScrollView` 时建议显式传入。
    pub wrap_width: Option<u16>,
    // 是否拆分超过宽度的长词/长串。默认 `true`，适合 CJK 正文和长日志。
    pub break_words: Option<bool>,
    // 是否把自身高度设为换行后的行数。默认 `true`。
    pub auto_height: Option<bool>,
}

// 自动换行文本组件。
pub struct WrappedText {
    paragraph: Paragraph<'static>,
    line_count: u16,
}

impl WrappedText {
    fn from_props(props: &WrappedTextProps) -> Self {
        let wrap_width = props
            .wrap_width
            .filter(|width| *width > 0)
            .unwrap_or(match props.width {
                Constraint::Length(width) if width > 0 => width,
                _ => DEFAULT_WRAP_WIDTH,
            });
        let wrapped = wrap_text(&props.text, wrap_width, props.break_words.unwrap_or(true));
        let line_count = wrapped_line_count(&wrapped);

        Self {
            paragraph: Paragraph::new(wrapped)
                .style(props.style)
                .scroll((props.scroll.x, props.scroll.y))
                .alignment(props.alignment),
            line_count,
        }
    }

    fn line_count(&self) -> u16 {
        self.line_count
    }
}

impl Component for WrappedText {
    type Props<'a> = WrappedTextProps;

    fn new(props: &Self::Props<'_>) -> Self {
        Self::from_props(props)
    }

    fn update(
        &mut self,
        props: &mut Self::Props<'_>,
        _hooks: Hooks,
        updater: &mut ComponentUpdater,
    ) {
        *self = Self::from_props(props);

        let mut layout_style = props.layout_style();
        if props.auto_height.unwrap_or(true) {
            layout_style.height = Constraint::Length(self.line_count());
        }
        updater.set_layout_style(layout_style);
    }

    fn draw(&mut self, drawer: &mut ComponentDrawer<'_, '_>) {
        drawer.render_widget(&self.paragraph, drawer.area);
    }
}

fn wrap_text(text: &str, width: u16, break_words: bool) -> String {
    let options = textwrap::Options::new(width.max(1) as usize).break_words(break_words);
    textwrap::fill(text, options)
}

fn wrapped_line_count(text: &str) -> u16 {
    text.lines().count().max(1).min(u16::MAX as usize) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_count_uses_wrap_width() {
        let props = WrappedTextProps {
            text: "alpha beta gamma".into(),
            wrap_width: Some(5),
            ..Default::default()
        };
        let text = WrappedText::from_props(&props);

        assert_eq!(text.line_count(), 3);
    }

    #[test]
    fn line_count_falls_back_to_length_width() {
        let props = WrappedTextProps {
            text: "alpha beta".into(),
            width: Constraint::Length(5),
            ..Default::default()
        };
        let text = WrappedText::from_props(&props);

        assert_eq!(text.line_count(), 2);
    }
}
