//! Markdown 流式输出示例。
//!
//! 模拟 LLM 逐字符输出场景：内容逐步"打字"出现，Markdown 实时解析渲染。
//! 按 `q` 退出，`a` 自动流式，`j/k` 手动步进，`f` 全部展示。

use ratatui_kit::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    ratatui::layout::{Constraint, Direction},
};
use std::time::Duration;

const CONTENT: &str = r#"# 流式 Markdown 演示

## 什么是流式输出？

流式输出模拟 LLM 或网络数据**逐步到达**的场景。

内容不是一次性出现，而是**逐字符**或**逐词**地展示。

---

## 代码块

流式场景下，代码块内容也会逐步填充：

```rust
async fn stream_content() {
    for ch in text.chars() {
        print!("{ch}");
    }
}
```

---

## 列表

- 列表项也会
- 一个接一个地
- 逐渐出现

---

## 表格

| 特性   | 状态   |
|--------|--------|
| 标题   | ✅     |
| 代码   | ✅     |
| 列表   | ✅     |
| 表格   | ✅     |

"#;

const CHARS_PER_TICK: usize = 1;
const TICK_MS: u64 = 50;

#[tokio::main]
async fn main() {
    element!(App)
        .fullscreen()
        .await
        .expect("Failed to run the application");
}

fn slice_by_chars(s: &str, count: usize) -> &str {
    match s.char_indices().nth(count) {
        Some((pos, _)) => &s[..pos],
        None => s,
    }
}

#[component]
fn App(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut exit = hooks.use_exit();
    let total_chars = CONTENT.chars().count();
    let streamed_chars = hooks.use_state(|| total_chars);
    let auto_mode = hooks.use_state(|| false);
    let streamed = streamed_chars.get();
    let auto = auto_mode.get();
    let display = slice_by_chars(CONTENT, streamed);

    // 自动流式：后台定时器
    let sc = streamed_chars.clone();
    let am_clone = auto_mode.clone();
    let mut started = hooks.use_state(|| false);
    hooks.use_effect(
        move || {
            if !auto {
                started.set(false);
                return;
            }
            if started.get() {
                return;
            }
            started.set(true);
            let mut sc = sc.clone();
            let am = am_clone.clone();
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(Duration::from_millis(TICK_MS)).await;
                    if !am.get() {
                        break; // auto 关闭则停止
                    }
                    let cur = sc.get();
                    if cur < total_chars {
                        sc.set((cur + CHARS_PER_TICK).min(total_chars));
                    }
                }
            });
        },
        (auto,),
    );

    let hint = if streamed >= total_chars {
        " q 退出 | s 流式重播 | a 自动流式".to_string()
    } else if auto {
        format!(
            " q 退出 | a 停止 | j/k 手动 | f 全部 | {streamed}/{total_chars} 字符",
        )
    } else {
        format!(
            " q 退出 | a 自动 | j/k 手动 | f 全部 | {streamed}/{total_chars} 字符",
        )
    };

    let mut sc_handler = streamed_chars.clone();
    let mut am = auto_mode.clone();
    hooks.use_event_handler(EventScope::Current, EventPriority::Normal, move |event| {
        if let Event::Key(key) = event
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Char('q') => {
                    exit();
                    return EventResult::Consumed;
                }
                KeyCode::Char('s') => {
                    sc_handler.set(3);
                    return EventResult::Consumed;
                }
                KeyCode::Char('a') => {
                    am.set(!am.get());
                    return EventResult::Consumed;
                }
                KeyCode::Char('j') => {
                    let cur = sc_handler.get();
                    if cur < total_chars {
                        sc_handler.set((cur + CHARS_PER_TICK).min(total_chars));
                    }
                    return EventResult::Consumed;
                }
                KeyCode::Char('k') => {
                    let cur = sc_handler.get();
                    sc_handler.set(cur.saturating_sub(CHARS_PER_TICK).max(3));
                    return EventResult::Consumed;
                }
                KeyCode::Char('f') => {
                    sc_handler.set(total_chars);
                    return EventResult::Consumed;
                }
                _ => {}
            }
        }
        EventResult::Ignored
    });

    element!(
        ScrollView(
            flex_direction: Direction::Vertical,
            scroll_bars: ScrollBars {
                vertical_scrollbar_visibility: ScrollbarVisibility::Always,
                ..Default::default()
            },
        ) {
            Markdown(content: display)
            View(height: Constraint::Length(1)) {
                Text(text: String::new())
            }
            View(height: Constraint::Length(1)) {
                Text(text: hint)
            }
        }
    )
}
