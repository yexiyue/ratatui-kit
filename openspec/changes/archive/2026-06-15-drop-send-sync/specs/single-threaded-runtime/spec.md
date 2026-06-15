## ADDED Requirements

### Requirement: 核心 trait 不要求 Send + Sync

`Component`、`AnyComponent`、`Hook`、`Props` SHALL NOT 要求 `Send` 或 `Sync`。`Props` SHALL 为**安全** trait（非 `unsafe`），由 `#[derive(Props)]` 生成安全 impl。

#### Scenario: 持有非 Send 数据的组件可用
- **WHEN** 一个组件的 props 或状态持有非 `Send` 的 ratatui 类型（如 `Block`/`Paragraph`）
- **THEN** 无需任何 `unsafe impl Send/Sync` 或包装类型即可作为组件/props 使用

### Requirement: future 内联 poll、不要求 Send

`use_future` 接受的 future SHALL NOT 要求 `Send`（`F: Future + 'static`），内部以 `LocalBoxFuture` 持有，并在 `poll_change` 中内联 poll。

#### Scenario: 非 Send future 可用
- **WHEN** `use_future` 的 async 块捕获了非 `Send` 的状态句柄
- **THEN** 仍可正常注册并被渲染循环内联轮询

### Requirement: 消除为绕过 Send 而生的 unsafe 与包装

框架内 SHALL NOT 残留「仅为满足 Send/Sync」的 `unsafe impl Send/Sync`；`SendBlock` 包装类型 SHALL 移除，承载可选边框的字段改用 `Option<Block<'static>>`。

#### Scenario: 边框字段为原生 Option<Block>
- **WHEN** 组件 props 需要可选边框
- **THEN** 字段类型为 `Option<Block<'static>>`，调用方可写裸 `Block`（经 `.into()` 自动 `Some`）或 `Some(..)`/`None`

#### Scenario: 无残留 Send/Sync 断言
- **WHEN** 全库检索 `unsafe impl Send`/`unsafe impl Sync`
- **THEN** 数量为 0

### Requirement: 运行时行为不变

去 Send 化 MUST NOT 改变运行时行为：渲染输出、事件处理、`use_future`/响应式更新与改造前一致；examples 正常运行。

#### Scenario: examples 与现有测试不回归
- **WHEN** 改造后跑 examples 与现有 23 单测 + trybuild + 四件套
- **THEN** 全部通过，行为一致
