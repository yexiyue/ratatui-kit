use crate::{AnyElement, element, prelude::Fragment};
use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect},
    style::Style,
    text::{Line, Text as RataText},
    widgets::{Paragraph, Widget},
};
use ratatui_kit_macros::{Props, component};
use std::ops::{Deref, DerefMut};

#[derive(Clone, Default)]
pub struct TextParagraph<'a> {
    inner: Paragraph<'a>,
}

// ratatui 0.30 起 `Paragraph` 内含 `Option<Block>`,而 `Block` 因新增阴影效果
// (`Arc<dyn CellEffect>`)不再 Send + Sync;但 `Props` 要求 Send + Sync。
// 与 `SendBlock` 同理:ratatui-kit 渲染单线程、所构造段落不挂自定义阴影效果,
// 故对该 newtype 断言 Send + Sync 是安全的。
// Safety: 见上方说明。
unsafe impl Send for TextParagraph<'_> {}
unsafe impl Sync for TextParagraph<'_> {}

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

// 让 TextParagraph 自身成为可渲染 widget,从而可经 `$expr` 直接嵌入元素树
// (`WidgetAdapter` 要求 widget 为 Send + Sync,裸 `Paragraph` 0.30 起不满足,故用本包装)。
impl Widget for TextParagraph<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.inner.render(area, buf);
    }
}

// 让 `&TextParagraph` 也成为 Widget,以匹配 WidgetAdapter 的 `for<'a> &'a T: Widget`
// 约束(去 clone 后按引用渲染)。`&Paragraph` 0.30 起本就是 Widget,直接转发。
impl Widget for &TextParagraph<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        (&self.inner).render(area, buf);
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
    pub style: Style,
    pub alignment: ratatui::layout::Alignment,
    pub scroll: Position,
    /// 是否换行(trim)。可直接传 `bool`(自动 `Some`)或 `Option<bool>`。
    pub wrap: Option<bool>,
}

#[component]
pub fn Text(props: &TextProps) -> impl Into<AnyElement<'static>> {
    let paragraph = props
        .text
        .inner
        .clone()
        .style(props.style)
        .scroll((props.scroll.x, props.scroll.y))
        .alignment(props.alignment);

    let paragraph = if let Some(wrap) = props.wrap {
        paragraph.wrap(ratatui::widgets::Wrap { trim: wrap })
    } else {
        paragraph
    };

    // 包成 Send + Sync 的 TextParagraph 再嵌入(裸 Paragraph 0.30 起非 Send,无法走 WidgetAdapter)。
    let paragraph = TextParagraph::from(paragraph);

    element! {
        Fragment{
            $paragraph
        }
    }
}
