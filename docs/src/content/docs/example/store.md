---
title: "全局状态管理"
sidebar:
    order: 6
---

本案例演示了如何在 Rust 终端应用中，利用 ratatui-kit 的 Store 全局状态管理机制，实现跨页面共享和同步数据。用户可以在不同页面间切换，计数器和输入框的内容始终保持同步。通过本示例，你可以学习到 Store 的定义与使用、全局状态的读写、页面间数据共享，以及如何构建响应式的终端多页面应用。

```rust
use ratatui::{
    style::{Style, Stylize},
    text::Line,
};
use ratatui_kit::ratatui;
use ratatui_kit::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    ratatui::layout::Constraint,
};

#[derive(Store)]
pub struct CounterAndTextInput {
    pub count: i32,
    pub value: String,
}

impl Default for CounterAndTextInput {
    fn default() -> Self {
        Self {
            count: 0,
            value: String::new(),
        }
    }
}

#[tokio::main]
async fn main() {
    let routes = routes! {
        "/" => HomePage,
        "/counter" => CounterPage,
        "/input" => InputPage,

    };

    element!(RouterProvider(
        routes:routes,
        index_path:"/",
    ))
    .fullscreen()
    .await
    .expect("Failed to run the application");
}

#[component]
fn HomePage(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let store = &COUNTER_AND_TEXT_INPUT_STORE;
    let (count, value) = use_stores!(store.count, store.value);
    let mut navigate = hooks.use_navigate();
    hooks.use_events(move |event| {
        if let Event::Key(key_event) = event {
            if key_event.kind == KeyEventKind::Press {
                match key_event.code {
                    KeyCode::Char('1') => navigate.push("/counter"),
                    KeyCode::Char('2') => navigate.push("/input"),

                    _ => {}
                }
            }
        }
    });
    element!(
        Border(
            style:Style::default().blue(),
            height:Constraint::Length(10),
            gap:1,
            top_title:Line::from("🏠 Store 全局状态仪表盘").centered().bold(),
        ){
            $Line::from(format!("全局计数: {}", count.get()))
            $Line::from(format!("全局输入: {}", value.read().as_str()))
            $Line::from("1. 计数器页面 (Counter)")
            $Line::from("2. 文本输入页面")
        }
    )
}

#[component]
fn CounterPage(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let store = &COUNTER_AND_TEXT_INPUT_STORE;
    let mut count = use_stores!(store.count);
    let value = use_stores!(store.value);
    let mut navigate = hooks.use_navigate();
    hooks.use_future(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            count += 1;
        }
    });
    hooks.use_events(move |event| {
        if let Event::Key(key_event) = event {
            if key_event.kind == KeyEventKind::Press && key_event.code == KeyCode::Esc {
                navigate.back();
            }
        }
    });
    element!(
        Border(
            style:Style::default().green(),
            height:Constraint::Length(6),
            top_title:Line::from("计数器页面 (ESC 返回)").centered(),
        ){
            $Line::from(format!("全局输入: {}", value.read().as_str()))
            $Line::styled(
                format!("Counter: {}", count.get()),
                Style::default().fg(ratatui::style::Color::Green).bold(),
            ).centered().bold().underlined()
        }
    )
}

#[component]
fn InputPage(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let store = &COUNTER_AND_TEXT_INPUT_STORE;
    let (mut value, count) = use_stores!(store.value, store.count);
    let mut navigate = hooks.use_navigate();
    hooks.use_events(move |event| {
        if let Event::Key(key_event) = event {
            if key_event.kind == KeyEventKind::Press && key_event.code == KeyCode::Esc {
                navigate.back();
            }
        }
    });
    element!(
        Border(
            style:Style::default().cyan(),
            height:Constraint::Length(7),
            top_title:Line::from("文本输入页面 (ESC 返回)").centered(),
        ){
            $Line::from(format!("全局计数: {}", count.get()))
            TextArea(
                value: value.read().to_string(),
                is_focus:true,
                on_change: move |new_value: String| {
                    value.set(new_value);
                },
                multiline: true,
                cursor_style: Style::default().on_cyan(),
                placeholder: Some("请输入内容...".to_string()),
                placeholder_style: Style::default().cyan(),
            )
        }
    )
}
```

运行结果如下:
![store](/ratatui-kit-website/example/store.gif)
