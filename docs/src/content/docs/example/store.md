---
title: "全局状态管理"
sidebar:
    order: 6
---

本案例演示如何用 ratatui-kit 的 Atom 全局原子在不同页面之间共享状态。计数器和输入框分别订阅同一组全局 Atom，任一页面写入后，订阅组件会自动刷新。

```rust
use ratatui::{style::{Style, Stylize}, text::Line};
use ratatui_kit::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    ratatui::layout::Constraint,
};

static COUNT: Atom<i32> = Atom::new(|| 0);
static VALUE: Atom<String> = Atom::new(String::new);

#[component]
fn HomePage(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let count = hooks.use_atom(&COUNT);
    let value = hooks.use_atom(&VALUE);
    let mut navigate = hooks.use_navigate();

    hooks.use_events(move |event| {
        if let Event::Key(key_event) = event
            && key_event.kind == KeyEventKind::Press
        {
            match key_event.code {
                KeyCode::Char('1') => navigate.push("/counter"),
                KeyCode::Char('2') => navigate.push("/input"),
                _ => {}
            }
        }
    });

    element!(
        Border(
            style: Style::default().blue(),
            height: Constraint::Length(10),
            gap: 1,
            top_title: Line::from("Atom 全局状态仪表盘").centered().bold(),
        ) {
            Text(text: format!("全局计数: {}", count.get()))
            Text(text: format!("全局输入: {}", value.read().as_str()))
            Text(text: "1. 计数器页面")
            Text(text: "2. 文本输入页面")
        }
    )
}

#[component]
fn CounterPage(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut count = hooks.use_atom(&COUNT);

    hooks.use_future(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            count += 1;
        }
    });

    element!(Text(text: format!("Counter: {}", count.get())))
}
```

完整代码见仓库中的 `examples/store.rs`。

运行结果如下:
![store](/ratatui-kit-website/example/store.gif)
