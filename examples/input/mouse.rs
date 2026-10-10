//! 鼠标/指针交互演示:按钮(含 disabled)、遮罩点击关闭、拖拽分隔条、hover 高亮。
//!
//! 运行:`cargo run --example mouse`
//! 需要终端鼠标捕获(本示例已在入口以 `RunOptions { mouse: true }` 启用);
//! 按住 Shift 可临时走终端本地选择。`q` 退出。

use ratatui_kit::{
    crossterm::event::{Event, KeyCode, MouseButton, MouseEventKind},
    prelude::*,
    ratatui::{
        layout::{Constraint, Direction, Flex, Margin},
        style::{Color, Style},
    },
};

#[tokio::main]
async fn main() {
    element!(App)
        .fullscreen_with(RunOptions { mouse: true })
        .await
        .expect("Failed to run the application");
}

#[component]
fn App(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let count = hooks.use_state(|| 0usize);
    let modal_open = hooks.use_state(|| false);
    let split = hooks.use_state(|| 30u16);
    let mut exit = hooks.use_exit();

    // 键盘:q 退出。
    hooks.use_event_handler(EventScope::Current, EventPriority::Normal, move |event| {
        if let Event::Key(key) = event
            && key.code == KeyCode::Char('q')
        {
            exit();
        }
        EventResult::Ignored
    });

    // 遮罩点击关闭:handler 注册在弹窗层(`Layer(layer)`),hit_test=false 收该层所有左键 Down。
    // 弹窗内按钮先命中并消费(冒泡序子先于父);未被消费的点击(遮罩/面板空白)落到这里 → 关闭。
    let layer = hooks.use_input_layer(modal_open.get(), true);
    let mut modal_open_for_close = modal_open;
    hooks.use_event_handler_with_options(
        EventScope::Layer(layer),
        EventPriority::Low,
        EventOptions { hit_test: false },
        move |event| {
            if let Event::Mouse(m) = event
                && m.kind == MouseEventKind::Down(MouseButton::Left)
            {
                modal_open_for_close.set(false);
                EventResult::Consumed
            } else {
                EventResult::Ignored
            }
        },
    );

    let modal_open_now = modal_open.get();
    let split_now = split.get();
    let count_now = count.get();

    element!(View(
        flex_direction: Direction::Vertical,
        justify_content: Flex::Start,
        margin: Margin::new(2, 1),
    ) {
        View(height: Constraint::Length(1)) {
            Text(text: format!("count = {count_now}   (drag the divider, hover the zone, q to quit)"))
        }
        Toolbar(count: count, modal_open: modal_open)
        View(height: Constraint::Length(1))
        SplitPane(split: split, at: split_now)
        View(height: Constraint::Fill(1))
        HoverZone
        Modal(
            open: modal_open_now,
            layer: Some(layer),
            width: Constraint::Percentage(50),
            height: Constraint::Length(8),
        ) {
            ModalPanel(count: count)
        }
    })
}

// 工具栏:三个变体按钮 + 一个 disabled。
#[derive(Default, Props)]
struct ToolbarProps {
    count: Option<State<usize>>,
    modal_open: Option<State<bool>>,
}

#[component]
fn Toolbar(props: &ToolbarProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    // 外部 state 与本组件正交:传外部 state 复用,不传则内部自建(与 Select/Table 一致)。
    let count = props.count.unwrap_or_else(|| hooks.use_state(|| 0usize));
    let modal_open = props
        .modal_open
        .unwrap_or_else(|| hooks.use_state(|| false));
    let count_now = count.get();
    let mut count_for_inc = count;
    let mut count_for_dec = count;
    let mut count_for_reset = count;
    let mut modal_for_open = modal_open;
    element!(View(
        flex_direction: Direction::Horizontal,
        justify_content: Flex::Start,
        height: Constraint::Length(3),
    ) {
        Button(label: "Inc".to_string(), on_press: move |()| {
            count_for_inc.set(count_for_inc.get() + 1)
        })
        Button(
            label: "Dec".to_string(),
            variant: ButtonVariant::Secondary,
            on_press: move |()| count_for_dec.set(count_for_dec.get().saturating_sub(1)),
        )
        Button(
            label: "Reset".to_string(),
            variant: ButtonVariant::Secondary,
            disabled: count_now == 0,
            on_press: move |()| count_for_reset.set(0),
        )
        Button(
            label: "Open modal".to_string(),
            variant: ButtonVariant::Danger,
            on_press: move |()| modal_for_open.set(true),
        )
    })
}

// 拖拽分隔条:左右两栏,中间 1 列分隔条,按住左键拖动调整分栏位置。
#[derive(Default, Props)]
struct SplitPaneProps {
    split: Option<State<u16>>,
    at: u16,
}

#[component]
fn SplitPane(props: &SplitPaneProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let split = props.split.unwrap_or_else(|| hooks.use_state(|| 30u16));
    let at = props.at.clamp(10, 60);
    element!(View(flex_direction: Direction::Horizontal) {
        View(width: Constraint::Length(at), justify_content: Flex::Center) {
            Text(text: "left pane".to_string())
        }
        Divider(split: split)
        View(width: Constraint::Fill(1), justify_content: Flex::Center) {
            Text(text: "right pane".to_string())
        }
    })
}

// 分隔条:use_drag 三态调整分栏;悬停高亮。
#[derive(Default, Props)]
struct DividerProps {
    split: Option<State<u16>>,
}

#[component]
fn Divider(props: &DividerProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let hovered = hooks.use_hover();
    let mut split = props.split.unwrap_or_else(|| hooks.use_state(|| 30u16));
    let style = if hovered {
        Style::new().fg(Color::Yellow)
    } else {
        Style::new().fg(Color::Gray)
    };
    hooks.use_drag(move |d| {
        if (10..=60).contains(&d.to.x) {
            split.set(d.to.x);
        }
    });
    element!(View(width: Constraint::Length(1)) {
        Text(text: "│".to_string(), style: Some(style))
    })
}

// hover 高亮区:use_hover 反馈进入/离开。
#[component]
fn HoverZone(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let hovered = hooks.use_hover();
    let (label, style) = if hovered {
        (
            "  << pointer inside >>  ",
            Style::new().bg(Color::Blue).fg(Color::White),
        )
    } else {
        ("   hover me with mouse   ", Style::new().fg(Color::Blue))
    };
    element!(View(width: Constraint::Length(26), justify_content: Flex::Center) {
        Text(text: label.to_string(), style: Some(style))
    })
}

// 弹窗面板:Close 按钮消费点击;面板空白处不消费 → 冒泡到遮罩关闭。
#[derive(Default, Props)]
struct ModalPanelProps {
    count: Option<State<usize>>,
}

#[component]
fn ModalPanel(props: &ModalPanelProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let count = props.count.unwrap_or_else(|| hooks.use_state(|| 0usize));
    let mut count_for_inc = count;
    element!(View(justify_content: Flex::Center, margin: Margin::new(2, 1)) {
        Text(text: format!("count = {}", count.get()))
        Button(label: "+1 here".to_string(), on_press: move |()| {
            count_for_inc.set(count_for_inc.get() + 1)
        })
        Text(text: "(click outside or panel blank to dismiss)".to_string())
    })
}
