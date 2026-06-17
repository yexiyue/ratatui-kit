# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 开发工作流

**IMPORTANT**：执行任何开发任务（编写代码、修改配置、添加依赖、改 feature 门控）前，必须先调用 `/dev-workflow` skill。它会加载项目知识库（`dev-notes/knowledge/`）中的最佳实践和踩坑记录，并在开发完成后引导更新知识库。

知识库主题：

- `dev-notes/knowledge/toolchain.md` — cargo 命令矩阵、feature flags 门控、lefthook/CI、打 tag 触发的 CI 发布(git-cliff 生成 CHANGELOG)、单元测试与「编译即基线」约定
- `dev-notes/knowledge/runtime-architecture.md` — Element/Component、协调(reconciliation)、渲染循环 + Waker 响应式、布局/透明布局
- `dev-notes/knowledge/hooks-and-state.md` — Hook 顺序规则、自定义 Hook 的 Sealed 约定、use_state vs 全局 Atom、State/AtomState 运算符重载
- `dev-notes/knowledge/macros-and-props.md` — 过程宏(element!/#[component] 等)、Props 类型擦除、ratatui Block props、AnyProps unsafe

## 项目概述

Ratatui Kit 是一个基于 [ratatui](https://github.com/ratatui-org/ratatui) 的 Rust 终端 UI 组件化开发框架，借鉴 React 生态（组件、props、hooks、context、路由、全局状态），构建在 tokio 异步运行时之上。注释、文档与提交信息以中文为主。

## 仓库结构

Cargo workspace（edition 2024，resolver 3）：

- [crates/ratatui-kit/](crates/ratatui-kit/) — 框架主库（发布为 `ratatui-kit`）。
- [crates/ratatui-kit-macros/](crates/ratatui-kit-macros/) — 过程宏库（`element!`、`#[component]`、`#[derive(Props)]`、`routes!`、`#[with_layout_style]`）。
- 根 crate `ratatui-kit-examples` 仅承载 [examples/](examples/)，通过 dev-dependency 以 `features = ["full"]` 引入主库。

## 常用命令

测试 / lint / 格式化 / 文档（与 [lefthook.yaml](lefthook.yaml) 的 pre-commit 及 [CI.yaml](.github/workflows/CI.yaml) 一致，提交前会自动执行；`RUSTDOCFLAGS="-D warnings"` 是 CI 的设置，lefthook 的 docs job 未带此环境变量）：

```bash
cargo test --locked --all-features --workspace --lib --tests --examples
cargo clippy --all-targets --all-features --workspace -- -D warnings
cargo fmt --all --check                       # rustfmt.toml: tab_spaces=4
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items --all-features --workspace --examples
```

运行示例（examples 已通过 dev-dependency 启用 full 特性，无需额外 flag）：

```bash
cargo run --example counter      # 其它：custom_list hello_world input list modal router scrollview store
# 注：textarea 示例随 textarea 特性下线已禁用（examples/textarea.rs.disabled）
```

发布由「打 tag 触发 CI」驱动：改 `crates/<crate>/Cargo.toml` 版本 → commit → `git tag <crate>-v<version>` → `git push --tags`，[.github/workflows/CD.yaml](.github/workflows/CD.yaml) 据标签 `cargo publish` 并（仅主库标签）用 git-cliff（[cliff.toml](cliff.toml)）生成 CHANGELOG 与 GitHub Release（弃用了 release.sh）。详见 [dev-notes/knowledge/toolchain.md](dev-notes/knowledge/toolchain.md)。

## 特性开关（feature flags）

主库默认 **不启用** 任何特性，按需开启。多数高级组件被特性门控，开发时改动这些模块需用 `--all-features` 或对应特性才能编译到：

- `router` → `RouterProvider`/`Outlet`、`routes!` 宏、`use_router`/`use_navigate`（依赖 `regex`）
- `atom` → 全局状态 `Atom`、`AtomState`、`use_atom`
- `input` → `Input` 组件（`tui-input`）
- `tree` → `TreeSelect` 组件（`tui-tree-widget`）
- `full` → 上述全部

宏库有独立的 `router` 特性，由主库的同名特性透传；`atom` 是纯主库 feature，无宏库透传。

**`textarea` 特性已下线**：随 ratatui 0.30 迁移，底层 `tui-textarea`（最新 0.7.0 钉死 ratatui ^0.29）尚无 0.30 兼容版，故移除了 `textarea` feature/依赖/example。组件源码隔离保留在 [components/textarea.rs](crates/ratatui-kit/src/components/textarea.rs)（未声明为模块、未接入树），待依赖支持 0.30 后恢复接入即可。todo.md 另计划用 `tui-input` 重写 textarea 以支持自动换行——改动前先确认方向。

## 核心架构

整体是一套「声明式 Element → 实例化组件树 → 异步渲染循环」的运行时，模仿 React 的协调（reconciliation）模型。理解以下几个文件即可把握全貌。

### 1. Element vs Component（声明 vs 实例）

- `Element<T>`（[element/mod.rs](crates/ratatui-kit/src/element/mod.rs)）是**轻量声明**：`{ key, props }`，每次渲染重新创建。`element!` 宏生成它，`AnyElement` 是类型擦除版本。
- `Component` trait（[component/mod.rs](crates/ratatui-kit/src/component/mod.rs)）是**行为定义**：`new` / `update` / `draw` / `calc_children_areas` / `poll_change`。
- `InstantiatedComponent`（[component/instantiated_component.rs](crates/ratatui-kit/src/component/instantiated_component.rs)）是**持久化的树节点**，持有 component 实例、hooks 列表、子节点 `Components`、`LayoutStyle`。这是状态真正存活的地方。

### 2. 协调（reconciliation）

`ComponentUpdater::update_children`（[render/updater.rs](crates/ratatui-kit/src/render/updater.rs)）按 `ElementKey` + 组件 `TypeId` 复用上一帧的 `InstantiatedComponent`，命中则复用（保留 hooks/状态），否则新建。这是「同一 key 同一类型 → 状态保留」语义的来源，类似 React 的 key diff。

### 3. 渲染循环与响应式

[render/tree.rs](crates/ratatui-kit/src/render/tree.rs) 的 `render_loop`：
```
loop { render(); if should_exit || ctrl_c break; select(component.wait(), terminal.wait()).await }
```
`render()` 先自顶向下 `update`（运行组件函数体、跑 hooks、协调子树），再 `terminal.draw` 自顶向下 `draw`。然后 `select` 在「组件树有变化」与「终端有事件」之间等待，任一就绪即重渲染。

「组件树有变化」由 `poll_change` 聚合驱动：组件、子节点、hooks 三路 `poll_change` 任一 `Ready` 即唤醒。响应式状态（`use_state` 的 `State<T>`、全局 `AtomState<T>`，均基于 `generational-box`）在写入时把存的 `Waker` 唤醒，从而打破 `select` 的阻塞触发下一帧。**关键：UI 不是命令式重绘，而是状态变更经 Waker 唤醒渲染循环。**

### 4. 布局

`LayoutStyle`（[render/layout_style.rs](crates/ratatui-kit/src/render/layout_style.rs)）= `flex_direction / justify_content / gap / margin / offset / width / height`，直接映射到 ratatui 的 `Layout`/`Constraint`。`Component::calc_children_areas` 默认按 flex 切分子区域，可重写实现自定义布局（如 `ScrollView`、`Modal`）。

**透明布局（transparent layout）**：`#[component]` 宏生成的函数组件会调用 `set_transparent_layout(true)`，使其不占据独立布局节点、直接继承首个子组件的 `LayoutStyle`。因此函数组件本质是「透传包装器」，布局属性需写在它返回的根元素上。

### 5. Hooks 系统

[hooks/mod.rs](crates/ratatui-kit/src/hooks/mod.rs)：`Hooks` 管理器按**调用顺序**（`hook_index`）索引 hooks，首帧 `push`、后续帧按序 `downcast` 取回——因此必须遵守 React 式「Hook 调用顺序稳定」规则（勿放进条件/循环）。`Hook` trait 暴露生命周期钩子：`poll_change` / `pre|post_component_update` / `pre|post_component_draw` / `on_drop`。

内置：`use_state` `use_future` `use_async_state` `use_event_handler`（取代旧 `use_events`/`use_local_events`；配 `use_input_layer`）`use_context` `use_memo` `use_effect`/`use_async_effect` `use_insert_before` `use_terminal_size`/`use_previous_size` `use_exit` `use_on_drop`，以及特性门控的 `use_router`（`use_navigate`/`use_route`/`use_params`）、`use_atom`。

**自定义 hook 约定**：定义实现 `Hook` 的结构体管理状态，再用 `pub trait UseXxx: private::Sealed` 暴露 API（`Sealed` 仅对 `Hooks` 实现，禁止外部实现），方法内通过 `self.use_hook(|| ...)` 注册。

### 6. 状态管理的两套体系

- **局部状态** `use_state`（[hooks/use_state.rs](crates/ratatui-kit/src/hooks/use_state.rs)）：每组件独立 `Owner`，随组件卸载释放。
- **全局状态** Atom（[atom/mod.rs](crates/ratatui-kit/src/atom/mod.rs)，`atom` 特性）：进程级 `LazyLock<Owner>` + `Atom<T>` 的 `OnceLock` 惰性初始化。用 `static COUNT: Atom<i32> = Atom::new(|| 0);` 声明全局原子，在组件内用 `hooks.use_atom(&COUNT)` 订阅。两者的 `State`/`AtomState` 都实现了 `Copy` 与算术运算符重载（`+=` 等会触发变更通知），运算符由 [reactive_ops.rs](crates/ratatui-kit/src/reactive_ops.rs) 统一生成。

`use_atom` 会在 atom 参数变化或组件卸载时清理旧 waker；写自定义参数化 Hook 时也要记得后续帧同步参数，不要只依赖 `use_hook(|| ...)` 的首帧初始化。

### 7. Context、终端与宏

- **Context**（[context.rs](crates/ratatui-kit/src/context.rs)）：`ContextStack` 维护作用域栈供 `use_context` 向上查找；`ContextProvider` 组件注入。`SystemContext` 控制 `exit()`。注意 `with_context` 内用 `transmute` 缩短生命周期——是 unsafe 但受栈即进即出保护。
- **Terminal**（[terminal/](crates/ratatui-kit/src/terminal/)）：封装 crossterm event-stream，提供 `wait()`（异步等事件）与 ctrl-c 检测。
- **过程宏**（[crates/ratatui-kit-macros/src/](crates/ratatui-kit-macros/src/)）：
  - `#[component]`（[component.rs](crates/ratatui-kit-macros/src/component.rs)）把 `fn Foo(hooks, props) -> impl Into<AnyElement>` 重写为单元结构体 + `Component` 实现，函数体搬进 `implementation`，在 `update` 中执行并把返回 element 作为唯一子节点。参数仅识别 `props`/`hooks`（及 `_` 前缀变体）。
  - `element!`：JSX 风格声明式宏。`Comp(prop: val) { children }` 构造子树；子节点块支持一等 `if`/`if let`/`for`/`match` 控制流；`{ expr }` 可内嵌任意返回 `Option`/`Vec`/`impl Iterator<Item = Element>`/`Element` 的 Rust 表达式；`widget(expr)` / `stateful(widget, state)` 通过 adapter 组件桥接 ratatui 原生 widget。
  - `#[with_layout_style]`：给 Props 结构体注入布局字段并生成 `layout_style()`，是组件获得布局能力的标准方式（见 [components/view.rs](crates/ratatui-kit/src/components/view.rs)）。

### 关键约定与陷阱

- `lib.rs` 末尾 `extern crate self as ratatui_kit;` 让库内代码也能使用本库宏（宏展开生成 `::ratatui_kit::...` 路径）。`prelude` 模块汇出常用项，示例统一 `use ratatui_kit::prelude::*;`。
- `AnyProps`（[props.rs](crates/ratatui-kit/src/props.rs)）用类型擦除裸指针 + 手动 drop 在借用/拥有两种 props 间转换，`downcast_*_unchecked` 是 unsafe，依赖协调阶段已校验 `TypeId`。
- props 必须实现安全 trait `Props`，通过 `#[derive(Props)]` 生成；无 props 的组件用 `NoProps`。
- **ratatui 0.30 起 `Block` 不再 `Send + Sync`**（内含阴影效果）。框架级 `Send + Sync` 已移除，因此 props 中可直接持有 `Option<Block<'static>>`，不要恢复旧的 `SendBlock` 包装。
- 仓库已有针对性**单元测试**（各模块 `#[cfg(test)] mod tests`：`element/key`、`multimap`、`use_state`/`atom`、`input`、`render`、`router`、`wrapped_text`/`list_state`/`virtual_list`/`tree_select` 等），由 CI/lefthook 的 `cargo test --lib --tests` 运行；覆盖面仍有限，故**所有 example + doctest 能编译**仍是回归底线（详见 `dev-notes/knowledge/toolchain.md`）。
