// AlertModal 组件：带输入互斥的提示弹窗。

use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Alignment, Constraint},
    style::{Color, Style},
    text::Line,
    widgets::Padding,
};
use ratatui_kit_macros::{Props, component, element};

use crate::{
    AnyElement, Handler, Hooks, UseEventHandler, UseInputLayer,
    components::{Border, Modal, Text, TextParagraph},
    input::{EventPriority, EventResult, EventScope},
};

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
    pub style: Style,
    pub border_style: Style,
    pub title_style: Style,
    pub message_style: Style,
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
            style: Style::default().dim(),
            border_style: Style::default().fg(Color::Yellow),
            title_style: Style::default().fg(Color::Yellow),
            message_style: Style::default(),
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

    element!(Modal(
        open: props.open,
        layer: Some(layer),
        width: props.width,
        height: props.height,
        style: props.style,
    ) {
        Border(
            border_style: props.border_style,
            top_title: props.title.clone().style(props.title_style).centered(),
            bottom_title: props.close_hint.clone(),
            padding: props.padding,
        ) {
            Text(
                text: props.message.clone(),
                style: props.message_style,
                alignment: Alignment::Center,
                wrap: true,
            )
        }
    })
}
