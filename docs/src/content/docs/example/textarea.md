---
title: "文本输入框"
sidebar:
    order: 2
---

本案例展示了如何在 Rust 终端应用中，利用 ratatui-kit 实现一个支持提交与退出的单行文本输入框。用户可以在输入框中输入内容，按 Enter 键将输入作为消息插入到界面上方，按 Esc 键则可直接退出程序。通过本示例，你将学会 ratatui-kit 输入组件的基本用法、事件响应、状态管理，以及如何动态插入一行内容。

```rust
use ratatui_kit::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    ratatui::{
        TerminalOptions, Viewport,
        layout::Constraint,
        style::{Style, Stylize},
        text::Line,
    },
};

#[tokio::main]
async fn main() {
    element!(MyTextInput)
        .render_loop(TerminalOptions {
            viewport: Viewport::Inline(4),
        })
        .await
        .expect("Failed to run the application");
}

#[component]
fn MyTextInput(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut value = hooks.use_state(String::new);

    let mut should_exit = hooks.use_state(|| false);

    let mut system_ctx = hooks.use_context_mut::<SystemContext>();
    let insert_before = hooks.use_insert_before();

    if should_exit.get() {
        system_ctx.exit();
    }

    hooks.use_events(move |event| {
        if let Event::Key(key_event) = event {
            if key_event.kind == KeyEventKind::Press {
                match key_event.code {
                    KeyCode::Esc => {
                        should_exit.set(true);
                    }
                    KeyCode::Enter => {
                        if !value.read().is_empty() {
                            insert_before
                                .render_before(Line::from(format!("message: {value}")), 1)
                                .finish();

                            value.set(String::new());
                        }
                    }
                    _ => {}
                }
            }
        }
    });

    element!(Border(
        height: Constraint::Length(4),
        style: Style::default().green(),
        bottom_title: Line::styled(
            "Press 'Enter' to submit, 'Esc' to exit",
            Style::default().yellow(),
        ).centered(),
    ) {
        TextArea(
            value: value.read().to_string(),
            is_focus: true,
            on_change: move |new_value: String| {
                value.set(new_value);
            },
            multiline: false,
            cursor_style: Style::default().on_green(),
            placeholder: Some("Type something...".to_string()),
            placeholder_style: Style::default().green(),
        )

    })
}
```

运行结果如下：
![textarea](/ratatui-kit-website/example/textarea.gif)
