# Ratatui Kit

Ratatui Kit 是一个基于 [ratatui](https://github.com/ratatui-org/ratatui) 的 Rust 终端 UI 组件化开发框架，灵感来源于 React 的组件模型，旨在让终端 UI 的开发变得更高效、可组合、易维护。

## 特性

- **组件化开发**：通过宏和 trait 实现类似 React 的组件、props、hooks 等机制。
- **Hooks 支持**：支持 use_state、use_future、use_events 等常用 hooks。
- **异步渲染**：天然支持 tokio 异步生态。
- **与 ratatui 无缝集成**：可直接使用 ratatui 的所有能力。
- **易于扩展**：支持自定义组件。

## 安装

在你的项目中添加依赖：

```bash
cargo add ratatui-kit
```

> 注意：本项目仍处于早期阶段，API 可能会有较大变动。

## 快速开始

一个简单的计数器示例：

```rust
use ratatui::{
    style::{Style, Stylize},
    text::Line,
};
use ratatui_kit::prelude::*;

#[tokio::main]
async fn main() {
    element!(Counter)
        .into_any()
        .fullscreen()
        .await
        .expect("Failed to run the application");
}

// 计数器组件
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
        $Line::styled(
            format!("Counter: {}", state),
            Style::default().fg(ratatui::style::Color::Green).bold(),
        )
        .centered()
        .bold()
        .underlined()
    )
}
```

更多示例见 [examples/](./examples)。

## 目录结构

- `packages/ratatui-kit/`：核心库
- `packages/ratatui-kit-macros/`：宏定义
- `examples/`：示例代码

## 鸣谢

`ratatui-kit` 的灵感来源于`iocraft` 和 `ratatui`，感谢它们提供的优秀基础。

## 贡献

欢迎 issue 和 PR！如有建议或 bug，请提交到 [GitHub Issues](https://github.com/yourname/ratatui-kit/issues)。

## License

MIT
