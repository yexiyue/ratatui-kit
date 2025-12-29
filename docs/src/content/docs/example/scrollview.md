---
title: "ScrollView"
sidebar:
    order: 3
---

本案例演示了如何在 Rust 终端应用中，利用 ratatui-kit 实现带滚动功能的内容展示。通过本示例，你可以学习到如何使用 ScrollView 组件实现内容的滚动浏览，以及如何结合事件处理和状态管理，构建交互式终端界面。

```rust
use ratatui::{
    style::{Style, Stylize},
    text::Line,
};
use ratatui_kit::ratatui::{self, layout::Constraint};
use ratatui_kit::{prelude::*, ratatui::layout::Direction};
use std::fs;

#[tokio::main]
async fn main() {
    element!(MarkdownReader)
        .fullscreen()
        .await
        .expect("Failed to run the application");
}

#[component]
fn MarkdownReader(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    // 读取 README.md 内容
    let lines = hooks.use_memo(
        || {
            let content = fs::read_to_string("README.md")
                .unwrap_or_else(|_| "无法读取 README.md".to_string());
            content.lines().map(|l| l.to_string()).collect::<Vec<_>>()
        },
        (),
    );

    let scroll_view_state = hooks.use_state(ScrollViewState::default);

    hooks.use_local_events(move |event| {
        scroll_view_state.write().handle_event(&event);
    });

    // 简单 markdown 渲染：标题高亮，其余普通文本
    let rendered: Vec<Line> = lines
        .into_iter()
        .map(|line| {
            if line.starts_with("# ") {
                Line::styled(line, Style::default().yellow().bold())
            } else if line.starts_with("## ") {
                Line::styled(line, Style::default().green().bold())
            } else if line.starts_with("### ") {
                Line::styled(line, Style::default().cyan())
            } else {
                Line::from(line)
            }
        })
        .collect();

    // 渲染每一行为 AnyElement
    let rendered_elements: Vec<AnyElement> = rendered
        .into_iter()
        .map(|line| {
            element!(View(height:Constraint::Length(1)){
                $line
            })
            .into_any()
        })
        .collect();

    element!(
        View(
            flex_direction:ratatui::layout::Direction::Vertical,
            gap:1,
        ){
            Border(
                border_style:Style::default().blue(),
                top_title:Some(Line::from("Markdown 文件阅读器 (ScrollView 示例)").centered()),
                bottom_title:Some(Line::from("上下/翻页滚动，Ctrl+C 退出").centered()),
            ){
                ScrollView(
                    flex_direction:Direction::Vertical,
                    scroll_view_state: scroll_view_state.get(),
                ){
                    #(rendered_elements)
                }
            }
        }
    )
}
```

运行结果如下:
![scrollview.gif](/ratatui-kit-website/example/scrollview.gif)
