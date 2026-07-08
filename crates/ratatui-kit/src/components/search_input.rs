// SearchInput 组件：带局部输入互斥的单行搜索框。
//
// 组件内部维护编辑态：默认按 `s` 进入输入层，输入层打开时会截断更低层 handler，
// `Enter` 提交、`Esc` 取消，避免背景列表/页面同时响应键盘事件。

use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{layout::Constraint, style::Style, text::Line};
use ratatui_kit_macros::{Props, component, element, with_layout_style};
use tui_input::backend::crossterm::EventHandler;

use crate::{
    AnyElement, ComponentTheme, Handler, Hooks, Palette, UseEffect, UseEventHandler, UseInputLayer,
    UseState, UseTheme,
    components::theme::resolve_style,
    components::{Border, Input},
    input::{EventPriority, EventResult, EventScope},
};

/// SearchInput 组件的主题 slot。状态色(active/success/error)、占位符、光标、状态文案样式
/// 全部从 [`Palette`] 派生。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchInputTheme {
    /// 非编辑态边框样式。
    pub border_style: Style,
    /// 编辑态(无校验结论)边框样式。
    pub active_border_style: Style,
    /// 校验通过边框样式。
    pub success_border_style: Style,
    /// 校验失败边框样式。
    pub error_border_style: Style,
    /// 输入文本样式。
    pub input_style: Style,
    /// 占位符样式。
    pub placeholder_style: Style,
    /// 光标块样式。
    pub cursor_style: Style,
    /// 校验通过状态文案样式。
    pub success_status_style: Style,
    /// 校验失败状态文案样式。
    pub error_status_style: Style,
}

impl ComponentTheme for SearchInputTheme {
    fn from_palette(palette: &Palette) -> Self {
        Self {
            border_style: Style::new().fg(palette.border),
            active_border_style: Style::new().fg(palette.border_active),
            success_border_style: Style::new().fg(palette.success),
            error_border_style: Style::new().fg(palette.error),
            input_style: Style::new().fg(palette.fg),
            placeholder_style: Style::new().fg(palette.placeholder),
            cursor_style: Style::new().bg(palette.accent).fg(palette.on_accent),
            success_status_style: Style::new().fg(palette.success),
            error_status_style: Style::new().fg(palette.error),
        }
    }
}

impl Default for SearchInputTheme {
    fn default() -> Self {
        Self::from_palette(&Palette::default())
    }
}

#[with_layout_style(margin, offset, width)]
#[derive(Props)]
pub struct SearchInputProps {
    // 外部受控值。
    pub value: String,
    // 空值时展示的占位文案。
    pub placeholder: String,
    // 是否允许进入编辑态。父级可用它在页面级状态中禁用搜索。
    pub is_editing: bool,
    // 从非编辑态进入编辑态的快捷键，默认 `s`。
    pub activate_key: KeyCode,
    // 输入变更回调。
    pub on_change: Handler<'static, String>,
    // 提交回调。返回 `false` 可阻止关闭输入态。
    pub on_submit: Handler<'static, String, bool>,
    // 清空回调。
    pub on_clear: Handler<'static, ()>,
    // 同步校验回调，返回 `(是否有效, 状态文案)`。
    pub validate: Handler<'static, String, (bool, String)>,
    // 提交成功后是否清空输入。
    pub clear_on_submit: bool,
    // `Esc` 取消时是否清空输入。
    pub clear_on_escape: bool,
    // 以下样式覆盖:`None` 用 `SearchInputTheme`,`Some(s)` 以 `theme.patch(s)` 覆盖。
    pub border_style: Option<Style>,
    pub active_border_style: Option<Style>,
    pub success_border_style: Option<Style>,
    pub error_border_style: Option<Style>,
    pub input_style: Option<Style>,
    pub placeholder_style: Option<Style>,
    pub cursor_style: Option<Style>,
    pub success_status_style: Option<Style>,
    pub error_status_style: Option<Style>,
}

impl Default for SearchInputProps {
    fn default() -> Self {
        Self {
            value: String::new(),
            placeholder: String::new(),
            is_editing: true,
            activate_key: KeyCode::Char('s'),
            on_change: Handler::default(),
            on_submit: Handler::default(),
            on_clear: Handler::default(),
            validate: Handler::default(),
            clear_on_submit: false,
            clear_on_escape: false,
            border_style: None,
            active_border_style: None,
            success_border_style: None,
            error_border_style: None,
            input_style: None,
            placeholder_style: None,
            cursor_style: None,
            success_status_style: None,
            error_status_style: None,
            margin: Default::default(),
            offset: Default::default(),
            width: Default::default(),
        }
    }
}

#[component]
pub fn SearchInput(
    props: &mut SearchInputProps,
    mut hooks: Hooks,
) -> impl Into<AnyElement<'static>> {
    let input = hooks.use_state(tui_input::Input::default);
    let mut editing = hooks.use_state(|| false);
    let mut status = hooks.use_state(String::new);
    let mut valid = hooks.use_state(|| None::<bool>);

    // 主题解析:每个 slot 铺底,对应 props 的 Option<Style> 在上 patch(None → 用主题)。
    let theme = hooks.use_component_theme::<SearchInputTheme>();
    let base_border_style = resolve_style(theme.border_style, props.border_style);
    let active_border_style = resolve_style(theme.active_border_style, props.active_border_style);
    let success_border_style =
        resolve_style(theme.success_border_style, props.success_border_style);
    let error_border_style = resolve_style(theme.error_border_style, props.error_border_style);
    let input_style = resolve_style(theme.input_style, props.input_style);
    let placeholder_style = resolve_style(theme.placeholder_style, props.placeholder_style);
    let cursor_style = resolve_style(theme.cursor_style, props.cursor_style);
    let success_status_style =
        resolve_style(theme.success_status_style, props.success_status_style);
    let error_status_style = resolve_style(theme.error_status_style, props.error_status_style);

    let is_enabled = props.is_editing;
    hooks.use_effect(
        move || {
            if !is_enabled {
                editing.set(false);
            }
        },
        is_enabled,
    );

    let value = props.value.clone();
    hooks.use_effect(
        move || {
            if input.read().value() != value {
                *input.write() = tui_input::Input::new(value);
            }
        },
        props.value.clone(),
    );

    let activate_key = props.activate_key;
    hooks.use_event_handler(EventScope::Current, EventPriority::High, move |event| {
        if !is_enabled || editing.get() {
            return EventResult::Ignored;
        }

        if let Event::Key(key) = event
            && key.kind == KeyEventKind::Press
            && key.code == activate_key
        {
            editing.set(true);
            return EventResult::Consumed;
        }

        EventResult::Ignored
    });

    let layer = hooks.use_input_layer(props.is_editing && editing.get(), true);
    let mut on_change = props.on_change.take();
    let mut on_submit = props.on_submit.take();
    let mut on_clear = props.on_clear.take();
    let mut validate = props.validate.take();
    let clear_on_submit = props.clear_on_submit;
    let clear_on_escape = props.clear_on_escape;
    let is_enabled = props.is_editing;

    hooks.use_event_handler(
        EventScope::Layer(layer),
        EventPriority::High,
        move |event| {
            if !is_enabled || !editing.get() {
                return EventResult::Ignored;
            }

            let Event::Key(key) = event else {
                return EventResult::Consumed;
            };
            if key.kind != KeyEventKind::Press {
                return EventResult::Consumed;
            }

            match key.code {
                KeyCode::Esc => {
                    if clear_on_escape {
                        input.write().reset();
                        valid.set(None);
                        status.set(String::new());
                        on_change(String::new());
                        on_clear(());
                    }
                    editing.set(false);
                    EventResult::Consumed
                }
                KeyCode::Enter => {
                    let submitted_value = input.read().value().to_string();
                    let should_close = if on_submit.is_default() {
                        true
                    } else {
                        on_submit(submitted_value)
                    };

                    if should_close {
                        if clear_on_submit {
                            input.write().reset();
                            valid.set(None);
                            status.set(String::new());
                            on_change(String::new());
                            on_clear(());
                        }
                        editing.set(false);
                    }

                    EventResult::Consumed
                }
                _ => {
                    input.write().handle_event(&Event::Key(key));
                    let next_value = input.read().value().to_string();
                    on_change(next_value.clone());

                    if validate.is_default() {
                        valid.set(None);
                        status.set(String::new());
                    } else {
                        let (next_valid, message) = validate(next_value);
                        valid.set(Some(next_valid));
                        status.set(message);
                    }

                    EventResult::Consumed
                }
            }
        },
    );

    let is_active = props.is_editing && editing.get();
    let status_title = if is_active && !status.read().is_empty() {
        let style = if valid.get() == Some(false) {
            error_status_style
        } else {
            success_status_style
        };
        Some(Line::styled(status.read().to_string(), style))
    } else {
        None
    };

    let border_style = if is_active {
        match valid.get() {
            Some(true) => success_border_style,
            Some(false) => error_border_style,
            None => active_border_style,
        }
    } else {
        base_border_style
    };

    element!(Border(
        margin: props.margin,
        offset: props.offset,
        width: props.width,
        height: Constraint::Length(3),
        border_style: border_style,
        top_title: status_title,
    ) {
        Input(
            input: input.read().clone(),
            cursor_style: cursor_style,
            placeholder: props.placeholder.clone(),
            placeholder_style: placeholder_style,
            style: input_style,
            hide_cursor: !is_active,
        )
    })
}
