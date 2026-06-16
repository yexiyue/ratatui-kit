//! 框架级输入互斥的 canonical demo:演示三条互斥路径 + 一个 footgun。
//!
//! ## 三条路径
//! - **① 背景列表(root 层)**:`use_event_handler(Current, Normal, ..)`。处理 ↑/↓(或 j/k)
//!   移动选中、`s` 进输入态、`m` 开弹窗、`q` 退出。返回 `Ignored`(纯观察 + 改本地状态)。
//! - **② 输入层(blocks_lower 截断背景)**:`use_input_layer(editing, true)` 开一个独占层,
//!   配 `use_event_handler(Layer(input_layer), High, ..)`。输入态下背景列表的 j/k 被截断,
//!   不再移动选中——这就是「输入互斥」。返回 `Consumed`。
//! - **③ 弹窗 Layer(h) 三件套**:`use_input_layer(modal_open, true)` 拿到 `modal_layer`,
//!   同时给 `use_event_handler(Layer(modal_layer), High, ..)` 和 `Modal(layer: Some(modal_layer))`。
//!   弹窗打开时独占输入,Esc/n 关、Enter/y 关,返回 `Consumed`。
//!
//! ## Footgun(关键注释见下方 ③ 处)
//! `modal_layer` 必须**同时**传给 `use_event_handler(Layer(..))` 和 `Modal(layer: ..)`。
//! 漏传任一:Modal 会自开一个新层截断父级注册的 `modal_layer` handler → 父级 handler 失聪。
//!
//! z-order 优先于 priority 的语义已由 input 模块单测 `layer_z_order_beats_priority` 覆盖,
//! 本 example 聚焦 ①②③ 三路互斥的演示。

use ratatui_kit::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    ratatui::{
        layout::{Constraint, Direction},
        style::{Style, Stylize},
        text::Line,
    },
};

#[tokio::main]
async fn main() {
    element!(App)
        .fullscreen()
        .await
        .expect("Failed to run the application");
}

#[component]
fn App(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let items = ["Apple", "Banana", "Cherry", "Durian", "Elderberry"];

    let mut selected = hooks.use_state(|| 0usize);
    let mut editing = hooks.use_state(|| false);
    let text = hooks.use_state(String::new);
    let mut modal_open = hooks.use_state(|| false);
    let mut exit = hooks.use_exit();

    // ① 背景列表(root 层):↑/↓ 或 j/k 移动、s 进输入态、m 开弹窗、q 退出。返回 Ignored。
    hooks.use_event_handler(EventScope::Current, EventPriority::Normal, move |event| {
        if let Event::Key(key) = event
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') if selected.get() > 0 => {
                    selected -= 1;
                }
                KeyCode::Down | KeyCode::Char('j') if selected.get() + 1 < items.len() => {
                    selected += 1;
                }
                KeyCode::Char('s') => editing.set(true),
                KeyCode::Char('m') => modal_open.set(true),
                KeyCode::Char('q') => exit(),
                _ => {}
            }
        }
        EventResult::Ignored
    });

    // ② 输入层:blocks_lower=true 截断背景。仅在 editing 时入栈(open=editing.get())。
    let input_layer = hooks.use_input_layer(editing.get(), true);
    hooks.use_event_handler(
        EventScope::Layer(input_layer),
        EventPriority::High,
        move |event| {
            // 非输入态时本 handler 即便被调用也不处理(且此时层未入栈,不会被调用)。
            if !editing.get() {
                return EventResult::Ignored;
            }
            if let Event::Key(key) = event
                && key.kind == KeyEventKind::Press
            {
                match key.code {
                    KeyCode::Esc | KeyCode::Enter => editing.set(false),
                    KeyCode::Char(c) => text.write().push(c),
                    KeyCode::Backspace => {
                        text.write().pop();
                    }
                    _ => {}
                }
            }
            // 输入态独占:消费事件,背景列表 j/k 不再移动选中(被截断)。
            EventResult::Consumed
        },
    );

    // ③ 弹窗 Layer(h) 三件套:modal_layer 必须同时给 use_event_handler(Layer) 和 Modal(layer:)。
    //    【FOOTGUN】漏传任一 → Modal 自开新层截断 modal_layer → 本 handler 失聪。
    let modal_layer = hooks.use_input_layer(modal_open.get(), true);
    hooks.use_event_handler(
        EventScope::Layer(modal_layer),
        EventPriority::High,
        move |event| {
            if !modal_open.get() {
                return EventResult::Ignored;
            }
            if let Event::Key(key) = event
                && key.kind == KeyEventKind::Press
            {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('n') => modal_open.set(false),
                    KeyCode::Enter | KeyCode::Char('y') => modal_open.set(false),
                    _ => {}
                }
            }
            EventResult::Consumed
        },
    );

    let status = if editing.get() {
        format!("[输入中] {}_  (Esc/Enter 结束)", text.read().as_str())
    } else {
        format!("文本: {}  (按 s 编辑)", text.read().as_str())
    };

    let cur = selected.get();
    let list_lines: Vec<Line<'static>> = items
        .iter()
        .enumerate()
        .map(|(i, name)| {
            if i == cur {
                Line::styled(format!("> {name}"), Style::default().black().on_cyan())
            } else {
                Line::from(format!("  {name}"))
            }
        })
        .collect();

    element!(
        View(
            flex_direction: Direction::Vertical,
            gap: 1,
        ) {
            // 背景列表:高亮 selected。
            Border(
                flex_direction: Direction::Vertical,
                border_style: Style::default().blue(),
                top_title: Line::from("输入互斥 Demo(j/k 移动 · s 编辑 · m 弹窗 · q 退出)").centered(),
            ) {
                // 一等 for:每行内联渲染。
                for (i, line) in list_lines.into_iter().enumerate() {
                    View(height: Constraint::Length(1), key: i) {
                        Text(text: line)
                    }
                }
            }
            // 输入框状态行。
            View(height: Constraint::Length(1)) {
                Text(text: status)
            }
            // ③ Modal:layer 必须传 modal_layer(与上方 use_event_handler(Layer(modal_layer)) 配对)。
            Modal(
                open: modal_open.get(),
                layer: Some(modal_layer),
                width: Constraint::Percentage(50),
                height: Constraint::Length(5),
                style: Style::default().dim(),
            ) {
                Border(
                    border_style: Style::default().yellow(),
                    top_title: Line::from("确认弹窗").centered().yellow(),
                ) {
                    Text(text: Line::from("确定吗? (y/Enter 确认 · n/Esc 取消)").centered())
                }
            }
        }
    )
}
