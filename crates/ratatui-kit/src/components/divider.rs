use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Paragraph, Widget},
};
use ratatui_kit_macros::{Props, component, with_layout_style};

use crate::{AnyElement, element, prelude::Fragment};

/// 水平分割线（分隔符）组件。
///
/// 渲染为一行由指定字符重复构成的水平线，类似 HTML 的 `<hr>`。
///
/// ## 用法示例
/// ```rust,no_run
/// element!(Divider(char: Some('─'), style_cfg: Some(Style::new().fg(Color::Cyan))))
/// ```
#[with_layout_style]
#[derive(Props)]
pub struct DividerProps {
    /// 分割线使用的字符。默认 `'─'`。
    pub char: Option<char>,
    /// 分割线的样式。默认暗灰色。
    pub style_cfg: Option<Style>,
}

impl Default for DividerProps {
    fn default() -> Self {
        Self {
            char: Some('─'),
            style_cfg: Some(Style::new().fg(Color::DarkGray)),
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

/// 分割线渲染 adapter。
///
/// 使用足够长的重复字符行填充，Widget 渲染时由 ratatui 在分配区域内自动截断。
#[derive(Clone)]
struct DividerLine {
    paragraph: Paragraph<'static>,
}

impl Widget for DividerLine {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.paragraph.render(area, buf);
    }
}

impl Widget for &DividerLine {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.paragraph.clone().render(area, buf);
    }
}

#[component]
pub fn Divider(props: &DividerProps) -> impl Into<AnyElement<'static>> {
    let ch = props.char.unwrap_or('─');
    let style = props.style_cfg.unwrap_or_default();

    let line = ch.to_string().repeat(256);
    let paragraph = Paragraph::new(line).style(style);

    element! {
        Fragment {
            widget(DividerLine { paragraph })
        }
    }
}
