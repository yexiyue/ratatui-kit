## Why

框架对 `Component`/`Hook`/`Props` 全程强制 `Send + Sync`，代价是 **23 个 `unsafe impl Send/Sync`** + 两个纯为绕过 Send 而生的包装类型（`SendBlock`、`TextParagraph`）。但该要求纯属 incidental：渲染路径**没有任何 `tokio::spawn`**，`use_future` 在 `poll_change` 里**内联 poll** future，顶层渲染 future 经 `block_on`（多线程 runtime 的 `block_on` 不要求 Send）。TUI 本质单线程，Send 并非真实需要。去掉它能大幅提升一致性与整洁度。

## What Changes

- **去 Send+Sync trait bound**：`Component: Any + Send + Sync + Unpin` → `Any + Unpin`；`AnyComponent` 同；`Hook: Unpin + Send` → `Unpin`；`Props: 由 unsafe trait + Send+Sync` → **安全 trait**（`#[derive(Props)]` 生成安全 impl）。
- **`use_future` 改非 Send**：`F: Future + Send` → `F: Future + 'static`，`BoxFuture` → `LocalBoxFuture`。
- **删除 23 个 `unsafe impl Send/Sync`**：`AnyProps`、`SystemContext`、`TextParagraph`、`SendBlock`、两个 adapter、`Route`/`Routes`。
- **删除 `SendBlock` 包装**：`tree_select`/`scroll_view` 的 `block` 字段改回 `Option<Block<'static>>`；`element!` 的 `.into()` + std `From<T> for Option<T>` 仍支持裸 `Block` 写法。
- **简化 `TextParagraph`**：去掉 `unsafe impl Send/Sync`（保留其 `From<&str>/Line/Text` 转换以维持 `Text(text:)` 人机工程）。
- examples 的 `#[tokio::main]` **无需改**（block_on 不要求 Send；先验证，若个别需要再调）。

## Capabilities

### New Capabilities
- `single-threaded-runtime`: 框架以单线程语义运行（future 内联 poll、不 spawn、不要求 Send+Sync）的契约——`Component`/`Hook`/`Props` 不再要求 Send+Sync，杜绝为绕过 Send 而生的 unsafe 断言与包装类型。

### Modified Capabilities
<!-- 无:openspec/specs/ 当前为空。 -->

## Impact

- **代码**：`component/mod.rs`、`hooks/mod.rs`、`hooks/use_future.rs`、`props.rs`、`ratatui-kit-macros/src/props.rs`、`context.rs`、`components/{text,send_block}.rs`、`components/adapter/{widget,stateful_widget}.rs`、`components/router/mod.rs`、`components/{tree_select,scroll_view}.rs`。
- **删除文件**：`components/send_block.rs`（及其模块声明/导出）。
- **公开 API**：`Props` 由 unsafe 变安全 trait；`SendBlock` 移除（改用 `Option<Block>`）——**破坏性**（用户已知，本轮不顾兼容）。
- **后续**：State/StoreState 的 `T: Send+Sync`（`SyncStorage`）可进一步换 `UnsyncStorage` 去约束 + 提速——并入 `redesign-store`（③）一并处理，本变更先拿下框架级 Send 与全部 unsafe workaround。
- **风险**：中——面广但机械；逐文件改 + 每步四件套验证，确保渲染/交互行为不变。
