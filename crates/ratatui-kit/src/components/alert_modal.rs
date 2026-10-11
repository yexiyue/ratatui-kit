// AlertModal 组件：带输入互斥的提示弹窗。
//
// 鼠标:键盘 handler 只 match `Key`(其余 Ignored 放行);未被消费的左键 Down
// (遮罩/面板空白)落到 Low + `hit_test:false` 的遮罩关闭 handler → `on_close`。
// 层本身独占(blocks_lower),放行不会让背景透过响应。

use crossterm::event::{Event, KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::{
    layout::{Alignment, Constraint},
    style::Style,
    text::Line,
    widgets::Padding,
};
use ratatui_kit_macros::{Props, component, element};
use std::sync::{Arc, Mutex};

use crate::{
    AnyElement, ComponentTheme, Handler, Hooks, Palette, UseEventHandler, UseInputLayer, UseTheme,
    components::theme::resolve_style,
    components::{Border, Modal, Text, TextParagraph},
    input::{EventOptions, EventPriority, EventResult, EventScope},
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
    // 键盘 handler 与遮罩关闭 handler 共享回调(Handler 不 Clone)。
    let on_close = Arc::new(Mutex::new(props.on_close.take()));

    // 键盘:模态独占键盘(所有 Key 事件消费);鼠标事件 Ignored 放行给遮罩关闭。
    hooks.use_event_handler(EventScope::Layer(layer), EventPriority::High, {
        let on_close = Arc::clone(&on_close);
        move |event| {
            let Event::Key(key) = event else {
                return EventResult::Ignored;
            };
            if key.kind == KeyEventKind::Press && close_keys.contains(&key.code) {
                (on_close.lock().unwrap())(());
            }
            EventResult::Consumed
        }
    });

    // 遮罩点击关闭:AlertModal 没有可点击子组件,任何未消费的左键 Down 都关闭。
    hooks.use_event_handler_with_options(
        EventScope::Layer(layer),
        EventPriority::Low,
        EventOptions { hit_test: false },
        {
            let on_close = Arc::clone(&on_close);
            move |event| {
                if let Event::Mouse(m) = event
                    && m.kind == MouseEventKind::Down(MouseButton::Left)
                {
                    (on_close.lock().unwrap())(());
                    return EventResult::Consumed;
                }
                EventResult::Ignored
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mouse_test::Mounted;
    use crossterm::event::{KeyCode, MouseButton, MouseEventKind};
    use std::sync::{Arc, Mutex};

    type SyncLog = Arc<Mutex<Vec<String>>>;

    #[derive(Clone, Default, Props)]
    struct HarnessProps {
        log: SyncLog,
    }

    #[component]
    fn Harness(props: &HarnessProps, _hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let log = props.log.clone();
        element!(AlertModal(
            open: true,
            on_close: move |_| log.lock().unwrap().push("close".into()),
        ))
    }

    // 点击遮罩/面板空白 → on_close;Esc → on_close。
    #[test]
    fn scrim_click_and_esc_close() {
        let log: SyncLog = Default::default();
        let mut m = Mounted::mount(element!(Harness(log: log.clone())), true);

        m.dispatch(MouseEventKind::Down(MouseButton::Left), 0, 0);
        assert_eq!(*log.lock().unwrap(), vec!["close".to_string()]);

        m.dispatch_key(KeyCode::Esc);
        assert_eq!(
            *log.lock().unwrap(),
            vec!["close".to_string(), "close".to_string()]
        );
    }

    // 滚轮不关闭(只放行、不误关)。
    #[test]
    fn wheel_over_modal_does_not_close() {
        let log: SyncLog = Default::default();
        let mut m = Mounted::mount(element!(Harness(log: log.clone())), true);

        m.dispatch(MouseEventKind::ScrollDown, 20, 4);
        assert!(log.lock().unwrap().is_empty());
    }
}
