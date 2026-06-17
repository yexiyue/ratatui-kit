// ShortcutInfoModal 组件：带输入互斥的快捷键帮助弹窗。

use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Constraint, Direction, Margin},
    style::{Color, Style},
    text::Line,
};
use ratatui_kit_macros::{Props, component, element};

use crate::{
    AnyElement, Handler, Hooks, UseEventHandler, UseInputLayer,
    components::{Border, Modal, ScrollView, Text, View},
    input::{EventPriority, EventResult, EventScope},
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ShortcutInfo {
    pub description: String,
    pub keys: String,
}

impl ShortcutInfo {
    pub fn new(description: impl Into<String>, keys: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            keys: keys.into(),
        }
    }
}

impl From<(&str, &str)> for ShortcutInfo {
    fn from((description, keys): (&str, &str)) -> Self {
        Self::new(description, keys)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ShortcutInfoSection {
    pub title: String,
    pub items: Vec<ShortcutInfo>,
}

impl ShortcutInfoSection {
    pub fn new<T>(title: impl Into<String>, items: impl IntoIterator<Item = T>) -> Self
    where
        T: Into<ShortcutInfo>,
    {
        Self {
            title: title.into(),
            items: items.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Props)]
pub struct ShortcutInfoModalProps {
    pub open: bool,
    pub title: Line<'static>,
    pub sections: Vec<ShortcutInfoSection>,
    pub close_hint: Option<Line<'static>>,
    pub close_keys: Vec<KeyCode>,
    pub on_close: Handler<'static, ()>,
    pub width: Constraint,
    pub height: Constraint,
    pub style: Style,
    pub border_style: Style,
    pub title_style: Style,
    pub section_title_style: Style,
    pub description_style: Style,
    pub key_style: Style,
}

impl Default for ShortcutInfoModalProps {
    fn default() -> Self {
        Self {
            open: false,
            title: Line::from("Shortcuts"),
            sections: Vec::new(),
            close_hint: Some(Line::from("Esc / I").centered()),
            close_keys: vec![KeyCode::Esc, KeyCode::Char('i'), KeyCode::Char('I')],
            on_close: Handler::default(),
            width: Constraint::Percentage(60),
            height: Constraint::Percentage(50),
            style: Style::default().dim(),
            border_style: Style::default(),
            title_style: Style::default(),
            section_title_style: Style::default(),
            description_style: Style::default(),
            key_style: Style::default().fg(Color::Yellow),
        }
    }
}

#[component]
pub fn ShortcutInfoModal(
    props: &mut ShortcutInfoModalProps,
    mut hooks: Hooks,
) -> impl Into<AnyElement<'static>> {
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
                return EventResult::Consumed;
            }
            EventResult::Ignored
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
        ) {
            ScrollView(margin: Margin::new(1, 1)) {
                for (section_index, section) in props.sections.clone().into_iter().enumerate() {
                    Border(
                        key: section_index,
                        height: Constraint::Length(section.items.len() as u16 + 2),
                        border_style: props.border_style,
                        top_title: Line::from(section.title).style(props.section_title_style).centered(),
                    ) {
                        View(flex_direction: Direction::Vertical) {
                            for (row_index, item) in section.items.into_iter().enumerate() {
                                View(
                                    key: row_index,
                                    height: Constraint::Length(1),
                                    flex_direction: Direction::Horizontal,
                                ) {
                                    View(width: Constraint::Percentage(55)) {
                                        Text(
                                            text: item.description,
                                            style: props.description_style,
                                        )
                                    }
                                    View(width: Constraint::Percentage(45)) {
                                        Text(
                                            text: Line::from(item.keys).right_aligned(),
                                            style: props.key_style,
                                        )
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    })
}
