<div align="center">

<img src="docs/src/assets/logo.svg" width="120" alt="Ratatui Kit logo" />

# Ratatui Kit

**Build component-driven terminal UIs in Rust — React-style components, hooks, props, router & global state. Powered by Ratatui.**

用 React 式的组件、Hooks、Props、路由与全局状态，在终端里构建声明式、响应式、异步的 TUI —— 构建于 Ratatui 与 Tokio 之上。

[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/yexiyue/ratatui-kit)
[![crates.io](https://img.shields.io/crates/v/ratatui-kit?logo=rust&color=E43717)](https://crates.io/crates/ratatui-kit)
[![Downloads](https://img.shields.io/crates/d/ratatui-kit?logo=rust)](https://crates.io/crates/ratatui-kit)
[![docs.rs](https://img.shields.io/docsrs/ratatui-kit?logo=docsdotrs)](https://docs.rs/ratatui-kit)
[![Website](https://img.shields.io/badge/website-ratatui--kit-3c8cba)](https://yexiyue.github.io/ratatui-kit/)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

**[文档](https://yexiyue.github.io/ratatui-kit/start/)** ·
**[快速开始](https://yexiyue.github.io/ratatui-kit/start/quick-start/)** ·
**[组件](https://yexiyue.github.io/ratatui-kit/components/)** ·
**[示例](https://yexiyue.github.io/ratatui-kit/examples/)**

</div>

---

## 简介

**Ratatui Kit** 是一个基于 [Ratatui](https://github.com/ratatui/ratatui) 的 Rust 终端 UI 组件化框架。它把 React 生态里那套成熟的心智模型——组件、Props、Hooks、Context、路由、全局状态——带到终端，让你像写前端一样写复杂 TUI。

如果你熟悉 React，你会立刻有宾至如归的感觉：`element!` 之于 JSX、`#[component]` 之于函数组件、`use_state` 之于 `useState`、`use_future` 之于 effect + async。底层渲染与布局由 [Ratatui](https://github.com/ratatui/ratatui) 提供，异步运行时由 [Tokio](https://github.com/tokio-rs/tokio) 提供，响应式状态由 [generational-box](https://crates.io/crates/generational-box) 支撑。

> Ratatui 给你画布与 widget；Ratatui Kit 给你组件、状态与协调（reconciliation），让复杂 TUI 像写 React 一样可维护。它是 Ratatui 之上的**组件化与响应式增强层**，而非替代。

<details>
<summary>目录</summary>

- [核心特性](#-核心特性)
- [快速开始](#-快速开始)
- [内置组件与 Hooks 一览](#-内置组件与-hooks-一览)
- [特性门控 feature flags](#-特性门控-feature-flags)
- [文档与示例](#-文档与示例)
- [设计理念与灵感来源](#-设计理念与灵感来源)
- [贡献](#-贡献)
- [许可证](#-许可证)

</details>

---

## ✨ 核心特性

- 🧩 **声明式组件** —— `element!` 提供 JSX 风格的声明式语法（一等 `if` / `if let` / `for` / `match` 控制流），`#[component]` 把 `fn Foo(hooks, props) -> impl Into<AnyElement>` 改写为组件。
- 🪝 **React 式 Hooks** —— `use_state` / `use_future` / `use_async_state` / `use_memo` / `use_effect` / `use_context` 等，按调用顺序管理状态与副作用。
- ♻️ **协调与状态保留** —— 区分 Element（轻量声明，每帧重建）、Component（行为）、InstantiatedComponent（持久树节点）；按 `ElementKey` + `TypeId` 复用上一帧节点，**同 key 同类型即保留 Hooks 与状态**（类似 React 的 key diff）。
- ⚡ **Waker 响应式渲染** —— 状态写入时唤醒 Waker，打破渲染循环 `select` 的阻塞，触发下一帧重渲染。**UI 不是命令式重绘，而是状态变更驱动**。
- 🚀 **异步原生** —— 渲染循环构建在 Tokio 之上：`render → draw → select(组件变化, 终端事件)`，可在组件里直接 `await`。
- 📐 **Flex 布局** —— `LayoutStyle`（`flex_direction` / `justify_content` / `gap` / `margin` / `offset` / `width` / `height`）直接映射到 Ratatui 的 `Layout` / `Constraint`；函数组件采用「透明布局」，继承首个子节点的布局。
- 🔀 **输入层与框架级互斥** —— 中央 `InputRuntime` + 输入层（`InputLayer`），弹窗 / 搜索框可独占键盘；事件经 `EventScope` / `EventPriority` / `EventResult` 分发，Hook 为 `use_event_handler`。
- 🗂️ **两套状态体系** —— 局部 `use_state`（每组件独立，随卸载释放）与全局 `Atom` / `use_atom`（进程级，`atom` 特性）；`State` / `AtomState` 均 `Copy` 且重载算术运算符，`count += 1` 即触发变更通知。
- 🧭 **内置路由** —— `RouterProvider` / `Outlet` / `routes!` 宏与 `use_navigate` / `use_route` / `use_params`（`router` 特性）。
- 🔌 **桥接原生 widget** —— 通过 `widget(expr)` / `stateful(widget, state)` 适配器，可直接嵌入任意 Ratatui 原生 widget。
- 🎛️ **特性门控、按需裁剪** —— 默认零特性（`default = []`），`router` / `atom` / `input` / `tree` / `virtual-list` 按需开启，只为你用到的能力付出体积。

---

## 🚀 快速开始

### 安装

```bash
cargo add ratatui-kit
```

或在 `Cargo.toml` 中按需启用特性：

```toml
[dependencies]
ratatui-kit = { version = "0.6.0", features = ["full"] }

# 需要一个 async runtime（文档与示例使用 tokio）
tokio = { version = "1", features = ["rt-multi-thread", "macros", "time"] }
```

> [!NOTE]
> 主库默认不启用任何特性。`router` / `atom` / `input` / `tree` / `virtual-list` 可按需开启，`full` 一次性启用全部。`textarea` 特性已随 Ratatui 0.30 迁移暂时下线（`tui-textarea` 暂无 0.30 兼容版）。

### 一个计数器

```rust
use ratatui_kit::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    ratatui::{
        layout::{Constraint, Direction, Flex},
        style::{Style, Stylize},
        text::Line,
    },
};

#[tokio::main]
async fn main() {
    element!(Counter)
        .fullscreen()
        .await
        .expect("Failed to run the application");
}

#[component]
fn Counter(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut count = hooks.use_state(|| 0_u64);

    // 异步副作用：每秒自增一次，写入即唤醒渲染循环
    hooks.use_future(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            count += 1;
        }
    });

    // 输入处理：按 q 退出
    let mut exit = hooks.use_exit();
    hooks.use_event_handler(EventScope::Current, EventPriority::Normal, move |event| {
        let Event::Key(key) = event else {
            return EventResult::Ignored;
        };
        if key.kind == KeyEventKind::Press
            && matches!(key.code, KeyCode::Char('q') | KeyCode::Char('Q'))
        {
            exit();
            return EventResult::Consumed;
        }
        EventResult::Ignored
    });

    element!(
        Center(width: Constraint::Length(48), height: Constraint::Length(9)) {
            Border(
                flex_direction: Direction::Vertical,
                justify_content: Flex::Center,
                border_style: Style::new().cyan(),
                top_title: Line::from(" ratatui-kit counter ").cyan().bold().centered(),
                bottom_title: Line::from(" q quit · Ctrl+C exit ").dark_gray().centered(),
            ) {
                Text(text: Line::styled(
                    format!("Counter: {:02}", count.get()),
                    Style::new().green().bold(),
                ).centered())
            }
        }
    )
}
```

运行它：

```bash
cargo run --example counter
```

---

## 🧩 内置组件与 Hooks 一览

> README 不罗列完整 API，详细签名与用法见[官方文档站](https://yexiyue.github.io/ratatui-kit/)与 [docs.rs](https://docs.rs/ratatui-kit)。

### 内置组件

| 组件 | 说明 | 所属特性 |
| --- | --- | --- |
| `View` / `Border` / `Center` / `Fragment` | 布局与容器基元 | 核心 |
| `Text` / `WrappedText` | 文本与自动换行文本 | 核心 |
| `Positioned` | 绝对定位 | 核心 |
| `Modal` / `ConfirmModal` / `AlertModal` / `ShortcutInfoModal` | 弹窗系列（可独占键盘） | 核心 |
| `Select` / `MultiSelect` | 单选 / 多选 | 核心 |
| `ScrollView` | 可滚动视图 | 核心 |
| `ContextProvider` | Context 注入 | 核心 |
| `Input` / `SearchInput` | 文本输入 / 搜索框 | `input` |
| `TreeSelect` | 树形选择 | `tree` |
| `VirtualList` | 虚拟列表 | `virtual-list` |
| `RouterProvider` / `Outlet` | 路由容器与出口 | `router` |

> 此外可通过 `widget(expr)` / `stateful(widget, state)` 适配器桥接任意 Ratatui 原生 widget。

### Hooks

| Hook | 说明 | 所属特性 |
| --- | --- | --- |
| `use_state` | 局部响应式状态 | 核心 |
| `use_future` / `use_async_state` | 异步任务 / 异步状态 | 核心 |
| `use_memo` / `use_effect` | 派生记忆值 / 副作用 | 核心 |
| `use_context` | 向上查找 Context | 核心 |
| `use_event_handler` | 注册输入事件处理器 | 核心 |
| `use_insert_before` / `use_size` | 在渲染区之前插入内容 / 读取尺寸 | 核心 |
| `use_exit` / `use_on_drop` | 退出应用 / 卸载回调 | 核心 |
| `use_navigate` / `use_route` / `use_params` | 路由导航、当前路由信息、路由参数 | `router` |
| `use_atom` | 订阅全局 Atom | `atom` |

### 过程宏

`element!` · `#[component]` · `#[derive(Props)]` · `routes!`（`router` 特性）· `#[with_layout_style]`

---

## 🎛️ 特性门控 feature flags

主库默认零特性，按需开启，只为用到的能力付出依赖与体积成本。

| 特性 | 解锁内容 | 额外依赖 |
| --- | --- | --- |
| `default` | （`[]`，不启用任何特性） | — |
| `router` | `RouterProvider` / `Outlet`、`routes!` 宏、`use_navigate` / `use_route` / `use_params` | `regex` |
| `atom` | `Atom` / `AtomState` / `use_atom` | — |
| `input` | `Input` / `SearchInput` 与 `tui_input` re-export | `tui-input` |
| `tree` | `TreeSelect` 与 `tui_tree_widget` re-export | `tui-tree-widget` |
| `virtual-list` | `VirtualList` 与 `tui_widget_list` re-export | `tui-widget-list` |
| `full` | 以上全部 | — |

---

## 📚 文档与示例

### 文档站

- [学习路径首页](https://yexiyue.github.io/ratatui-kit/start/)
- [快速开始](https://yexiyue.github.io/ratatui-kit/start/quick-start/)
- [安装与功能门控](https://yexiyue.github.io/ratatui-kit/start/installation/)
- [Hooks](https://yexiyue.github.io/ratatui-kit/core/hooks/)
- [状态模型](https://yexiyue.github.io/ratatui-kit/core/state/)
- [路由](https://yexiyue.github.io/ratatui-kit/core/routing/)
- [内置组件](https://yexiyue.github.io/ratatui-kit/components/)
- [示例](https://yexiyue.github.io/ratatui-kit/examples/)

还有 [DeepWiki](https://deepwiki.com/yexiyue/ratatui-kit) 可对仓库直接提问。

### 精选示例

```bash
cargo run --example counter        # 计数器：最小响应式状态 + 异步副作用
cargo run --example router         # 路由与嵌套 Outlet
cargo run --example atom_state     # 全局 Atom 状态
cargo run --example modal          # 弹窗与键盘独占
cargo run --example todo_app       # 综合：组件、状态、输入与路由的完整应用
```

<details>
<summary>展开全部示例（<code>cargo run --example &lt;name&gt;</code>）</summary>

```
hello_world          counter              async_state          atom_state
router               control_flow         input_mutex          input
search_input         scrollview           wrapped_text         modal
confirm_modal        alert_modal          shortcut_info_modal  select
multi_select         tree_select          virtual_list         virtual_multi_select
custom_widget        custom_hook          custom_provider      todo_app
```

部分示例需要对应特性（如 `input` / `tree` / `virtual-list` / `router`）；从仓库根运行示例已默认启用 `full`，无需额外 flag。

</details>

---

## 💡 设计理念与灵感来源

Ratatui Kit 的设计直接借鉴了 React，以及 Rust 生态里的 [iocraft](https://github.com/ccbrown/iocraft)（React-like TUI）与 Node 生态里的 [ink](https://github.com/vadimdemedes/ink)（React for CLI）。它把这套理念落到三句取舍上：

- **声明式** —— 用 `element!` 描述「UI 应该长什么样」，而不是命令式地一步步操作终端缓冲区。
- **响应式** —— 状态变更经 Waker 唤醒渲染循环，框架负责协调与最小重渲染，你不必手动触发重绘。
- **异步原生** —— 站在 Tokio 之上，异步任务、定时器、IO 都能直接融入组件生命周期。

与裸 Ratatui 相比，你获得了组件化、Hooks、状态与协调；而底层的画布、widget 与终端后端，依然是 Ratatui。我们站在巨人的肩膀上，致谢 [Ratatui](https://github.com/ratatui/ratatui) 与 [Tokio](https://github.com/tokio-rs/tokio)，以及 React、iocraft、ink 在设计上的启发。

---

## 🤝 贡献

欢迎提交 Issue 与 PR！

- Bug 反馈与功能建议请走 [GitHub Issues](https://github.com/yexiyue/ratatui-kit/issues)。
- 提交代码前请确保通过本地校验：

  ```bash
  cargo fmt --all --check
  cargo clippy --all-targets --all-features --workspace -- -D warnings
  cargo test --locked --all-features --workspace --lib --tests --examples
  ```

  仓库已配置 lefthook，pre-commit 会自动执行上述检查。

注释、文档与提交信息以中文为主。

---

## 📄 许可证

本项目以 [MIT](LICENSE) 许可证发布。
