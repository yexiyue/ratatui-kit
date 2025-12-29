---
title: "计数器"
sidebar:
    order: 1
---

本案例演示了如何使用 ratatui-kit 在 Rust 终端应用中实现一个简单的计数器。计数器会每秒自动加一，并实时在终端界面中展示当前数值。通过本示例，你可以学习到 ratatui-kit 的组件化写法、状态管理（use_state）、异步任务（use_future）以及如何在终端 UI 中进行动态内容渲染。

```rust
use ratatui::{
    style::{Style, Stylize},
    text::Line,
};
use ratatui_kit::ratatui::{
    self,
    layout::{Constraint, Flex},
};
use ratatui_kit::{prelude::*, ratatui::layout::Direction};

#[tokio::main]
async fn main() {
    element!(Counter)
        .fullscreen()
        .await
        .expect("Failed to run the application");
}

#[component]
fn Counter(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut state = hooks.use_state(|| 0);
    hooks.use_future(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            state += 1;
        }
    });

    element!(
        Border(
            flex_direction: Direction::Vertical,
            justify_content: Flex::Center,
        ){
            View(height:Constraint::Length(1)){
                $Line::styled(
                    format!("Counter: {state}"),
                    Style::default().fg(ratatui::style::Color::Green).bold(),
                )
                .centered()
                .bold()
            }
        }
    )
}
```

运行结果如下：
![counter.gif](/ratatui-kit-website/example/counter.gif)
