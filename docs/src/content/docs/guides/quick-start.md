---
title: "快速入门"
sidebar:
    order: 2
---

Ratatui Kit 让你像写前端一样高效开发终端 UI。下面带你快速体验从安装到第一个组件的完整流程。

## 安装依赖

在你的 Rust 项目中添加依赖：

```bash
cargo add ratatui-kit
```

如需使用路由、全局状态等高级功能，可在 `Cargo.toml` 中启用对应特性：

```toml
ratatui-kit = { version = "*", features = ["router", "store"] }
```

## 创建第一个组件

下面以计数器为例，演示组件定义、状态管理和异步更新的完整流程。

### 1. 定义组件

使用 `#[component]` 宏标记组件。组件函数支持 `hooks`（生命周期与状态管理）和 `props`（属性）参数：

```rust
#[component]
fn Counter(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    // ...
}
```

### 2. 管理状态与副作用

在组件内部，利用 `hooks` 管理本地状态和异步任务：

```rust
let mut state = hooks.use_state(|| 0);
hooks.use_future(async move {
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        state += 1;
    }
});
```

### 3. 渲染组件内容

用 `element!` 宏声明式渲染 UI。这里添加边框、设置布局方向和对齐方式：

```rust
element!(
    Border(
        flex_direction: Direction::Vertical,
        justify_content: Flex::Center,
    ){
        View(height: Constraint::Length(1)){
            $Line::styled(
                format!("Counter: {state}"),
                Style::default().green().bold(),
            )
            .centered()
            .bold()
        }
    }
)
```

> 注意：在 `element!` 宏中，ratatui 的原生 widget 需用 `$` 前缀。

### 4. 启动应用

在 `main` 函数中调用你的组件即可：

```rust
#[tokio::main]
async fn main() {
    element!(Counter)
        .fullscreen()
        .await
        .expect("Failed to run the application");
}
```

这样就完成了一个每秒自增的计数器组件，终端界面会自动刷新。

完整示例和运行效果请参考[计数器示例](https://yexiyue.github.io/ratatui-kit-website/example/counter/)
