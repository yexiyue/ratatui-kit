// AlertModal 组件：带输入互斥的提示弹窗。

use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Alignment, Constraint},
    style::Style,
    text::Line,
    widgets::Padding,
};
use ratatui_kit_macros::{Props, component, element};

use crate::{
    AnyElement, ComponentTheme, Handler, Hooks, Palette, UseEventHandler, UseInputLayer, UseTheme,
    components::theme::resolve_style,
    components::{Border, Modal, Text, TextParagraph},
    input::{EventPriority, EventResult, EventScope},
};

/// AlertModal 组件的主题 slot。提示语义 → 边框/标题取 `warning` 色;遮罩委托给 [`Modal`]。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlertModalTheme {
    /// 边框样式。
    pub border_style: Style,
    /// 标题样式。
    pub title_style: Style,
    /// 正文样式。
    pub message_style: Style,
}

impl ComponentTheme for AlertModalTheme {
    fn from_palette(palette: &Palette) -> Self {
        Self {
            border_style: Style::new().fg(palette.warning),
            title_style: Style::new().fg(palette.warning),
            message_style: Style::new().fg(palette.fg),
        }
    }
}

impl Default for AlertModalTheme {
    fn default() -> Self {
        Self::from_palette(&Palette::default())
    }
}

#[derive(Props)]
pub struct AlertModalProps {
    pub open: bool,
    pub title: Line<'static>,
    pub message: TextParagraph<'static>,
    pub close_hint: Option<Line<'static>>,
    pub close_keys: Vec<KeyCode>,
    pub on_close: Handler<'static, ()>,
    pub width: Constraint,
    pub height: Constraint,
    // 遮罩样式覆盖(透传给 `Modal`)。`None` 用 `ModalTheme`(默认 DIM)。
    pub style: Option<Style>,
    // 以下样式覆盖:`None` 用 `AlertModalTheme`,`Some(s)` 以 `theme.patch(s)` 覆盖。
    pub border_style: Option<Style>,
    pub title_style: Option<Style>,
    pub message_style: Option<Style>,
    pub padding: Padding,
}

impl Default for AlertModalProps {
    fn default() -> Self {
        Self {
            open: false,
            title: Line::from("Alert"),
            message: TextParagraph::from(""),
            close_hint: Some(Line::from("Esc / Enter").centered()),
            close_keys: vec![KeyCode::Esc, KeyCode::Enter],
            on_close: Handler::default(),
            width: Constraint::Percentage(50),
            height: Constraint::Length(6),
            style: None,
            border_style: None,
            title_style: None,
            message_style: None,
            padding: Padding::uniform(1),
        }
    }
}

#[component]
pub fn AlertModal(props: &mut AlertModalProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let layer = hooks.use_input_layer(props.open, true);
    let close_keys = props.close_keys.clone();
    let mut on_close = props.on_close.take();

    hooks.use_event_handler(
        EventScope::Layer(layer),
        EventPriority::High,
        move |event| {
            if let Event::Key(key) = event
                && key.kind == KeyEventKind::Press
                && close_keys.contains(&key.code)
            {
                on_close(());
            }
            EventResult::Consumed
        },
    );

    // 主题解析:每个 slot 铺底,对应 props 的 Option<Style> 在上 patch(None → 用主题)。
    let theme = hooks.use_component_theme::<AlertModalTheme>();
    let border_style = resolve_style(theme.border_style, props.border_style);
    let title_style = resolve_style(theme.title_style, props.title_style);
    let message_style = resolve_style(theme.message_style, props.message_style);

    element!(Modal(
        open: props.open,
        layer: Some(layer),
        width: props.width,
        height: props.height,
        style: props.style,
    ) {
        Border(
            border_style: border_style,
            top_title: props.title.clone().style(title_style).centered(),
            bottom_title: props.close_hint.clone(),
            padding: props.padding,
        ) {
            Text(
                text: props.message.clone(),
                style: message_style,
                alignment: Alignment::Center,
                wrap: true,
            )
        }
    })
}
