//! Markdown 内置组件示例。
//!
//! 展示 Markdown 组件已支持的全部特性：多级标题、行内样式、
//! 代码块（语法高亮）、有序/无序列表、表格、水平分割线。

use ratatui_kit::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    ratatui::layout::Direction,
};

const CONTENT: &str = r#"# Markdown 渲染示例

## 标题层级

### 三级标题

#### 四级标题

---

## 行内样式

这是 **粗体** 文本，这是 *斜体* 文本，~~这是删除线~~。

行内 `代码` 示例，以及 [链接](https://example.com) 文本。

---

## 代码块

带语言标识的代码块会自动语法高亮：

```rust
fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 0,
        1 => 1,
        _ => fibonacci(n - 1) + fibonacci(n - 2),
    }
}
```

无语言标识的代码块显示为纯文本：

```
$ cargo build --release
$ ./target/release/my-app
```

---

## 列表

无序列表：

- 项目 A
- 项目 B
- 项目 C

嵌套列表：

- 父项目
  - 子项目 A
  - 子项目 B
    - 孙子项目

有序列表：

1. 第一步
2. 第二步
3. 第三步

---

## 表格

表格支持列对齐：

| 左对齐 | 居中 | 右对齐 |
|:-------|:----:|-------:|
| Rust   |  🦀  | 系统编程 |
| Python |  🐍  | AI/数据科学 |
| Go     |  🐹  | 云原生 |

---

## 水平分割线

上面的标题和下面内容之间就是一条水平线。
"#;

#[tokio::main]
async fn main() {
    element!(App)
        .fullscreen()
        .await
        .expect("Failed to run the application");
}

#[component]
fn App(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut exit = hooks.use_exit();

    hooks.use_event_handler(EventScope::Current, EventPriority::Normal, move |event| {
        if let Event::Key(key) = event
            && key.kind == KeyEventKind::Press
            && key.code == KeyCode::Char('q')
        {
            exit();
            return EventResult::Consumed;
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
            Markdown(content: CONTENT)
        }
    )
}
