---
title: "JSON 编辑器"
sidebar:
    order: 4
---

本案例演示了如何在 Rust 终端应用中，利用 ratatui-kit 实现一个带 Modal 弹窗的 JSON 编辑与校验工具。用户可以在文本框中输入或粘贴 JSON 内容，程序会实时校验并格式化输入。按下 Tab 键可弹出 Modal，查看格式化结果或错误提示。通过本示例，你可以学习到 Modal 组件的用法、事件处理、状态管理以及如何实现终端下的交互式校验体验。

```rust
use ratatui::{
    style::{Style, Stylize},
    text::Line,
};
use ratatui_kit::prelude::*;
use ratatui_kit::ratatui;
use ratatui_kit::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    ratatui::widgets::Paragraph,
};
use serde_json;

#[tokio::main]
async fn main() {
    element!(JsonEditor)
        .into_any()
        .fullscreen()
        .await
        .expect("Failed to run the application");
}

#[component]
fn JsonEditor(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut json_text = hooks.use_state(|| String::from("{\n  \"key\": \"value\"\n}"));
    let mut open = hooks.use_state(|| false);
    let mut formatted = hooks.use_state(String::new);
    let mut error = hooks.use_state(String::new);

    // 实时解析 JSON
    hooks.use_effect(
        move || match serde_json::from_str::<serde_json::Value>(&json_text.read()) {
            Ok(val) => {
                let pretty = serde_json::to_string_pretty(&val).unwrap_or_default();
                formatted.set(pretty);
                error.set(String::new());
            }
            Err(e) => {
                formatted.set(String::new());
                error.set(e.to_string());
            }
        },
        [json_text.read().clone()],
    );

    // 事件处理：Tab 弹出 Modal
    hooks.use_events(move |event| {
        if let Event::Key(key_event) = event {
            if key_event.kind == KeyEventKind::Press && key_event.code == KeyCode::Tab {
                open.set(!open.get());
            }
        }
    });

    let info_line = if error.read().is_empty() {
        Line::styled(
            "JSON 格式正确",
            Style::default().green(),
        )
        .centered()
    } else {
        Line::styled(
            format!("JSON 错误: {}", error.read().as_str()),
            Style::default().red(),
        )
        .centered()
    };

    let modal_title = if error.read().is_empty() {
        Line::styled("JSON 格式化：", Style::default().green())
    } else {
        Line::styled("JSON 错误：", Style::default().red())
    };

    let modal_content = if error.read().is_empty() {
        Paragraph::new(formatted.read().clone())
    } else {
        Paragraph::new(error.read().clone())
    };

    element!(
        View{
            View{
                Border(
                    border_style:Style::default().blue(),
                    top_title:Some(info_line),
                    bottom_title:Some(Line::from("按 Tab 查看格式化/校验结果，Ctrl+C 退出").centered()),
                ){
                    TextArea(
                        value: json_text.read().to_string(),
                        is_focus: true,
                        on_change: move |new_value: String| {
                            json_text.set(new_value);
                        },
                        disable_keys: vec![Key::Tab],
                        multiline: true,
                        cursor_style: Style::default().on_blue(),
                        placeholder: Some("请输入 JSON...".to_string()),
                        placeholder_style: Style::default().blue(),
                        style: Style::default(),
                        line_number_style: Some(Style::default().dim())
                    )
                }
            }
            Modal(
                open:open.get(),
                width:ratatui::layout::Constraint::Percentage(60),
                height:ratatui::layout::Constraint::Percentage(60),
                style:Style::default().dim(),
            ){
                Border(
                    top_title:Some(Line::from("格式化/校验结果").centered().yellow()),
                    padding:ratatui::widgets::Padding::new(2,2,1,1),
                ) {
                    View(height:ratatui::layout::Constraint::Length(1),){
                        $modal_title
                    }
                    View{
                        $modal_content
                    }
                }
            }
        }
    )
}
```

运行结果如下:
![modal.gif](/ratatui-kit-website/example/modal.gif)
