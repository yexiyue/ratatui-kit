// ConfirmModal 组件：带输入互斥的确认弹窗。
//
// 组件内部自开独占输入层并把同一层传给 `Modal`，封装父级 handler + Modal
// 的三件套配对，避免背景组件处理确认弹窗期间的按键。
//
// 鼠标:键盘 handler 只 match `Key`(非键盘事件 Ignored 放行),按钮子组件
// (Normal,命中过滤)先收到点击;未被按钮消费的点击(遮罩/面板空白)落到
// Low + `hit_test:false` 的遮罩关闭 handler → `on_cancel`。层本身独占
// (`blocks_lower`),放行不会让背景透过响应。

use crossterm::event::{Event, KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Flex, Margin},
    style::{Modifier, Style},
    text::Line,
};
use ratatui_kit_macros::{Props, component, element};
use std::sync::{Arc, Mutex};

use crate::{
    AnyElement, ComponentTheme, Handler, Hooks, Palette, PointerKind, PointerOptions, UseEffect,
    UseEventHandler, UseHover, UseInputLayer, UsePointer, UseState, UseTheme,
    components::theme::resolve_style,
    components::{Border, Modal, Text, TextParagraph, View},
    input::{EventOptions, EventPriority, EventResult, EventScope},
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
    // 键盘 handler、遮罩关闭 handler 与两个按钮共享同一对回调(Handler 不 Clone;
    // 按钮的 props 回调要求 Send+Sync,故用 Arc<Mutex> 而非 Rc<RefCell>)。
    let on_confirm = Arc::new(Mutex::new(props.on_confirm.take()));
    let on_cancel = Arc::new(Mutex::new(props.on_cancel.take()));

    // 键盘:只消费 Key 事件(模态独占键盘);鼠标事件 Ignored 放行给本层的
    // 按钮与遮罩关闭 handler —— 层已 blocks_lower,放行不会漏到背景。
    hooks.use_event_handler(EventScope::Layer(layer), EventPriority::High, {
        let on_confirm = on_confirm.clone();
        let on_cancel = on_cancel.clone();
        move |event| {
            let Event::Key(key) = event else {
                return EventResult::Ignored;
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
                        (on_confirm.lock().unwrap())(());
                    } else {
                        (on_cancel.lock().unwrap())(());
                    }
                }
                KeyCode::Char('y') | KeyCode::Char('Y') => (on_confirm.lock().unwrap())(()),
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    (on_cancel.lock().unwrap())(())
                }
                _ => {}
            }

            EventResult::Consumed
        }
    });

    // 遮罩点击取消:Low + hit_test=false,按钮(Normal,命中过滤)先消费各自
    // 区域内的 Down;未被消费的左键 Down(遮罩/面板空白)落到这里 → on_cancel。
    hooks.use_event_handler_with_options(
        EventScope::Layer(layer),
        EventPriority::Low,
        EventOptions { hit_test: false },
        {
            let on_cancel = on_cancel.clone();
            move |event| {
                if let Event::Mouse(m) = event
                    && m.kind == MouseEventKind::Down(MouseButton::Left)
                {
                    (on_cancel.lock().unwrap())(());
                    return EventResult::Consumed;
                }
                EventResult::Ignored
            }
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
                        on_press: shared_handler(&on_cancel),
                    )
                    ConfirmButton(
                        label: props.confirm_text.clone(),
                        selected: confirm_selected,
                        width: button_width,
                        style: button_style,
                        selected_style: selected_button_style,
                        on_press: shared_handler(&on_confirm),
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
    on_press: Handler<'static, ()>,
}

/// 把共享回调包装成可传给子组件 props 的 `Handler`。
fn shared_handler(shared: &Arc<Mutex<Handler<'static, ()>>>) -> Handler<'static, ()> {
    let shared = Arc::clone(shared);
    Handler::from(move |()| (shared.lock().unwrap())(()))
}

#[component]
fn ConfirmButton(
    props: &mut ConfirmButtonProps,
    mut hooks: Hooks,
) -> impl Into<AnyElement<'static>> {
    let mut on_press = props.on_press.take();
    // 悬停视同选中(仅视觉强调,不改键盘的 confirm_selected 状态);
    // 左键 Down 即触发并消费 —— 与 Button 组件一致的终端惯例。
    let hovered = hooks.use_hover();
    hooks.use_pointer(PointerOptions::default(), move |ev| {
        if let PointerKind::Down(MouseButton::Left) = ev.kind {
            on_press(());
            EventResult::Consumed
        } else {
            EventResult::Ignored
        }
    });

    let selected = props.selected || hovered;
    let label_style = if selected {
        selected_button_label_style(props.selected_style)
    } else {
        props.style
    };
    let border_style = button_border_style(label_style);
    let label = if selected {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mouse_test::{Mounted, find_text};
    use crossterm::event::{KeyCode, MouseButton, MouseEventKind};
    use std::sync::{Arc, Mutex};

    type SyncLog = Arc<Mutex<Vec<String>>>;

    #[derive(Clone, Default, Props)]
    struct HarnessProps {
        log: SyncLog,
    }

    #[component]
    fn Harness(props: &HarnessProps, _hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let confirm_log = props.log.clone();
        let cancel_log = props.log.clone();
        element!(ConfirmModal(
            open: true,
            // 默认标题就是 "Confirm",会干扰 find_text 定位按钮,故换一个。
            title: Line::from("Prompt"),
            on_confirm: move |_| confirm_log.lock().unwrap().push("confirm".into()),
            on_cancel: move |_| cancel_log.lock().unwrap().push("cancel".into()),
        ))
    }

    fn mount(log: &SyncLog) -> Mounted {
        // 60x16:默认 40x10 时按钮行会溢出内区,标签画到命中盒外、点击落空。
        Mounted::mount_sized(element!(Harness(log: log.clone())), true, 60, 16)
    }

    fn click(m: &mut Mounted, col: u16, row: u16) {
        m.dispatch(MouseEventKind::Down(MouseButton::Left), col, row);
        m.dispatch(MouseEventKind::Up(MouseButton::Left), col, row);
    }

    // 点击确认按钮 → on_confirm;点击取消按钮 → on_cancel(左键 Down 即触发)。
    #[test]
    fn clicking_buttons_fires_matching_callback() {
        let log: SyncLog = Default::default();
        let mut m = mount(&log);

        let (cx, cy) = find_text(&m.buffer(), "Confirm").expect("确认按钮已渲染");
        click(&mut m, cx, cy);
        assert_eq!(*log.lock().unwrap(), vec!["confirm".to_string()]);

        let (x, y) = find_text(&m.buffer(), "Cancel").expect("取消按钮已渲染");
        click(&mut m, x, y);
        assert_eq!(
            *log.lock().unwrap(),
            vec!["confirm".to_string(), "cancel".to_string()]
        );
    }

    // 点击遮罩/面板空白 → on_cancel(未被按钮消费的左键 Down 落到 Low 级遮罩关闭)。
    #[test]
    fn scrim_click_cancels() {
        let log: SyncLog = Default::default();
        let mut m = mount(&log);

        click(&mut m, 0, 0); // 面板(居中 20x10)之外的遮罩
        assert_eq!(*log.lock().unwrap(), vec!["cancel".to_string()]);
    }

    // 滚轮在弹窗上滚动不触发任何回调(只放行、不误关)。
    #[test]
    fn wheel_over_modal_does_nothing() {
        let log: SyncLog = Default::default();
        let mut m = mount(&log);

        m.dispatch(MouseEventKind::ScrollDown, 20, 4);
        m.dispatch(MouseEventKind::ScrollUp, 20, 4);
        assert!(log.lock().unwrap().is_empty());
    }

    // 键盘语义回归:y 确认、Esc 取消、Enter 走当前选中(默认取消)。
    #[test]
    fn keyboard_semantics_unchanged() {
        let log: SyncLog = Default::default();
        let mut m = mount(&log);

        m.dispatch_key(KeyCode::Char('y'));
        assert_eq!(*log.lock().unwrap(), vec!["confirm".to_string()]);

        m.dispatch_key(KeyCode::Enter); // 初始选中取消按钮
        assert_eq!(
            *log.lock().unwrap(),
            vec!["confirm".to_string(), "cancel".to_string()]
        );

        m.dispatch_key(KeyCode::Esc);
        assert_eq!(
            *log.lock().unwrap(),
            vec![
                "confirm".to_string(),
                "cancel".to_string(),
                "cancel".to_string()
            ]
        );
    }
}
