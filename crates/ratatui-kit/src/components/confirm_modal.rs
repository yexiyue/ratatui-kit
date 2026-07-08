// ConfirmModal 组件：带输入互斥的确认弹窗。
//
// 组件内部自开独占输入层并把同一层传给 `Modal`，封装父级 handler + Modal
// 的三件套配对，避免背景组件处理确认弹窗期间的按键。

use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Flex, Margin},
    style::{Modifier, Style},
    text::Line,
};
use ratatui_kit_macros::{Props, component, element};

use crate::{
    AnyElement, ComponentTheme, Handler, Hooks, Palette, UseEffect, UseEventHandler, UseInputLayer,
    UseState, UseTheme,
    components::theme::resolve_style,
    components::{Border, Modal, Text, TextParagraph, View},
    input::{EventPriority, EventResult, EventScope},
};

/// ConfirmModal 组件的主题 slot。遮罩(背景变暗)委托给 [`Modal`] 的 `ModalTheme`,
/// 本 slot 只负责边框/标题/正文/按钮样式;选中按钮的 `BOLD` 由主题承接。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfirmModalTheme {
    /// 边框样式。
    pub border_style: Style,
    /// 标题样式。
    pub title_style: Style,
    /// 正文样式。
    pub content_style: Style,
    /// 未选中按钮样式。
    pub button_style: Style,
    /// 选中按钮样式(含 `BOLD`)。
    pub selected_button_style: Style,
}

impl ComponentTheme for ConfirmModalTheme {
    fn from_palette(palette: &Palette) -> Self {
        Self {
            border_style: Style::new().fg(palette.border),
            title_style: Style::new().fg(palette.fg),
            content_style: Style::new().fg(palette.fg),
            button_style: Style::new().fg(palette.fg),
            selected_button_style: Style::new().fg(palette.accent).add_modifier(Modifier::BOLD),
        }
    }
}

impl Default for ConfirmModalTheme {
    fn default() -> Self {
        Self::from_palette(&Palette::default())
    }
}

#[derive(Props)]
pub struct ConfirmModalProps {
    pub open: bool,
    pub title: Line<'static>,
    pub content: TextParagraph<'static>,
    pub confirm_text: String,
    pub cancel_text: String,
    pub on_confirm: Handler<'static, ()>,
    pub on_cancel: Handler<'static, ()>,
    pub width: Constraint,
    pub height: Constraint,
    // 遮罩样式覆盖(透传给 `Modal`)。`None` 用 `ModalTheme`(默认 DIM)。
    pub style: Option<Style>,
    // 以下样式覆盖:`None` 用 `ConfirmModalTheme`,`Some(s)` 以 `theme.patch(s)` 覆盖。
    pub border_style: Option<Style>,
    pub title_style: Option<Style>,
    pub content_style: Option<Style>,
    pub button_style: Option<Style>,
    pub selected_button_style: Option<Style>,
}

impl Default for ConfirmModalProps {
    fn default() -> Self {
        Self {
            open: false,
            title: Line::from("Confirm"),
            content: TextParagraph::from(""),
            confirm_text: String::from("Confirm"),
            cancel_text: String::from("Cancel"),
            on_confirm: Handler::default(),
            on_cancel: Handler::default(),
            width: Constraint::Percentage(50),
            height: Constraint::Length(10),
            style: None,
            border_style: None,
            title_style: None,
            content_style: None,
            button_style: None,
            selected_button_style: None,
        }
    }
}

#[component]
pub fn ConfirmModal(
    props: &mut ConfirmModalProps,
    mut hooks: Hooks,
) -> impl Into<AnyElement<'static>> {
    let mut confirm_selected = hooks.use_state(|| false);

    let open = props.open;
    hooks.use_effect(
        move || {
            if !open {
                confirm_selected.set(false);
            }
        },
        open,
    );

    let layer = hooks.use_input_layer(props.open, true);
    let mut on_confirm = props.on_confirm.take();
    let mut on_cancel = props.on_cancel.take();

    hooks.use_event_handler(
        EventScope::Layer(layer),
        EventPriority::High,
        move |event| {
            let Event::Key(key) = event else {
                return EventResult::Consumed;
            };
            if key.kind != KeyEventKind::Press {
                return EventResult::Consumed;
            }

            match key.code {
                KeyCode::Left | KeyCode::Right | KeyCode::Tab | KeyCode::BackTab => {
                    confirm_selected.set(!confirm_selected.get());
                }
                KeyCode::Enter => {
                    if confirm_selected.get() {
                        on_confirm(());
                    } else {
                        on_cancel(());
                    }
                }
                KeyCode::Char('y') | KeyCode::Char('Y') => on_confirm(()),
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => on_cancel(()),
                _ => {}
            }

            EventResult::Consumed
        },
    );

    let confirm_selected = confirm_selected.get();
    let button_width = confirm_button_width(&props.cancel_text, &props.confirm_text);

    // 主题解析:每个 slot 铺底,对应 props 的 Option<Style> 在上 patch(None → 用主题)。
    let theme = hooks.use_component_theme::<ConfirmModalTheme>();
    let border_style = resolve_style(theme.border_style, props.border_style);
    let title_style = resolve_style(theme.title_style, props.title_style);
    let content_style = resolve_style(theme.content_style, props.content_style);
    let button_style = resolve_style(theme.button_style, props.button_style);
    let selected_button_style =
        resolve_style(theme.selected_button_style, props.selected_button_style);

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
        ) {
            View {
                View(
                    height: Constraint::Fill(1),
                    margin: Margin::new(2, 2),
                ) {
                    Text(
                        text: props.content.clone(),
                        style: content_style,
                        alignment: Alignment::Center,
                        wrap: true,
                    )
                }
                View(
                    justify_content: Flex::SpaceAround,
                    height: Constraint::Length(3),
                    flex_direction: Direction::Horizontal,
                ) {
                    ConfirmButton(
                        label: props.cancel_text.clone(),
                        selected: !confirm_selected,
                        width: button_width,
                        style: button_style,
                        selected_style: selected_button_style,
                    )
                    ConfirmButton(
                        label: props.confirm_text.clone(),
                        selected: confirm_selected,
                        width: button_width,
                        style: button_style,
                        selected_style: selected_button_style,
                    )
                }
            }
        }
    })
}

#[derive(Default, Props)]
struct ConfirmButtonProps {
    label: String,
    selected: bool,
    width: u16,
    style: Style,
    selected_style: Style,
}

#[component]
fn ConfirmButton(props: &ConfirmButtonProps, _hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let label_style = if props.selected {
        selected_button_label_style(props.selected_style)
    } else {
        props.style
    };
    let border_style = button_border_style(label_style);
    let label = if props.selected {
        format!(" {} ", props.label)
    } else {
        props.label.clone()
    };

    element!(Border(
        width: Constraint::Length(props.width),
        height: Constraint::Length(3),
        border_style: border_style,
    ) {
        Text(
            text: Line::styled(label, label_style),
            alignment: Alignment::Center,
        )
    })
}

fn selected_button_label_style(style: Style) -> Style {
    let mut label_style = style;
    if let Some(bg) = style.bg {
        label_style.fg = Some(bg);
        label_style.bg = None;
    }
    label_style.add_modifier(Modifier::BOLD)
}

fn button_border_style(style: Style) -> Style {
    let mut border_style = style;
    if let Some(bg) = style.bg {
        border_style.fg = Some(bg);
        border_style.bg = None;
    }
    border_style
}

fn confirm_button_width(cancel_text: &str, confirm_text: &str) -> u16 {
    let label_width = cancel_text
        .chars()
        .count()
        .max(confirm_text.chars().count())
        .max(6);
    label_width.saturating_add(6).min(u16::MAX as usize) as u16
}
