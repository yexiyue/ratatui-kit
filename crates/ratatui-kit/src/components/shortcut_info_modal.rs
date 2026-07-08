// ShortcutInfoModal 组件：带输入互斥的快捷键帮助弹窗。

use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Constraint, Direction, Margin},
    style::Style,
    text::Line,
};
use ratatui_kit_macros::{Props, component, element};

use crate::{
    AnyElement, ComponentTheme, Handler, Hooks, Palette, UseEventHandler, UseInputLayer, UseTheme,
    components::theme::resolve_style,
    components::{Border, Modal, ScrollView, Text, View},
    input::{EventPriority, EventResult, EventScope},
};

/// ShortcutInfoModal 组件的主题 slot。快捷键取 `accent` 高亮,其余中性;遮罩委托给 [`Modal`]。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShortcutInfoModalTheme {
    /// 外框及分组框样式。
    pub border_style: Style,
    /// 标题样式。
    pub title_style: Style,
    /// 分组标题样式。
    pub section_title_style: Style,
    /// 快捷键描述样式。
    pub description_style: Style,
    /// 快捷键按键样式(高亮)。
    pub key_style: Style,
}

impl ComponentTheme for ShortcutInfoModalTheme {
    fn from_palette(palette: &Palette) -> Self {
        Self {
            border_style: Style::new().fg(palette.border),
            title_style: Style::new().fg(palette.fg),
            section_title_style: Style::new().fg(palette.fg),
            description_style: Style::new().fg(palette.fg),
            key_style: Style::new().fg(palette.accent),
        }
    }
}

impl Default for ShortcutInfoModalTheme {
    fn default() -> Self {
        Self::from_palette(&Palette::default())
    }
}

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
    // 遮罩样式覆盖(透传给 `Modal`)。`None` 用 `ModalTheme`(默认 DIM)。
    pub style: Option<Style>,
    // 以下样式覆盖:`None` 用 `ShortcutInfoModalTheme`,`Some(s)` 以 `theme.patch(s)` 覆盖。
    pub border_style: Option<Style>,
    pub title_style: Option<Style>,
    pub section_title_style: Option<Style>,
    pub description_style: Option<Style>,
    pub key_style: Option<Style>,
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
            style: None,
            border_style: None,
            title_style: None,
            section_title_style: None,
            description_style: None,
            key_style: None,
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

    // 主题解析:每个 slot 铺底,对应 props 的 Option<Style> 在上 patch(None → 用主题)。
    let theme = hooks.use_component_theme::<ShortcutInfoModalTheme>();
    let border_style = resolve_style(theme.border_style, props.border_style);
    let title_style = resolve_style(theme.title_style, props.title_style);
    let section_title_style = resolve_style(theme.section_title_style, props.section_title_style);
    let description_style = resolve_style(theme.description_style, props.description_style);
    let key_style = resolve_style(theme.key_style, props.key_style);

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
        ) {
            ScrollView(margin: Margin::new(1, 1)) {
                for (section_index, section) in props.sections.clone().into_iter().enumerate() {
                    Border(
                        key: section_index,
                        height: Constraint::Length(section.items.len() as u16 + 2),
                        border_style: border_style,
                        top_title: Line::from(section.title).style(section_title_style).centered(),
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
                                            style: description_style,
                                        )
                                    }
                                    View(width: Constraint::Percentage(45)) {
                                        Text(
                                            text: Line::from(item.keys).right_aligned(),
                                            style: key_style,
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
