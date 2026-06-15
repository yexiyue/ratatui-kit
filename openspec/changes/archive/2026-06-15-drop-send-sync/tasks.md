## 1. future 与核心 trait 去 Send

- [x] 1.1 `hooks/use_future.rs`：`F: Future + Send + 'static` → `F: Future + 'static`；`BoxFuture` → `LocalBoxFuture`；`.boxed()` → `.boxed_local()`（含 `use_async_effect` 等同源）
- [x] 1.2 `hooks/mod.rs`：`Hook: Unpin + Send` → `Hook: Unpin`
- [x] 1.3 `component/mod.rs`：`Component`/`AnyComponent` 去 `Send + Sync`（保留 `Any + Unpin`）

## 2. Props 改安全 trait

- [x] 2.1 `props.rs`：`pub unsafe trait Props: Send + Sync {}` → `pub trait Props {}`；删 `AnyProps` 的 `unsafe impl Send/Sync`
- [x] 2.2 `ratatui-kit-macros/src/props.rs`：`unsafe impl ... Props` → `impl ... Props`

## 3. 删 SendBlock + 边框字段回归 Option<Block>

- [x] 3.1 删除 `components/send_block.rs` + `components/mod.rs` 的模块声明/导出
- [x] 3.2 `components/tree_select.rs`：`block: SendBlock` → `Option<Block<'static>>`，`SendBlock::default()` → `None`，内部访问按 `Option<Block>` 调整
- [x] 3.3 `components/scroll_view/mod.rs`：同上（props 字段 + 组件结构体字段 + draw 内访问）

## 4. 其余 unsafe 清除

- [x] 4.1 `components/text.rs`：删 `TextParagraph` 的 `unsafe impl Send/Sync`（保留 newtype 与 From 转换、`&TextParagraph: Widget`）
- [x] 4.2 `components/adapter/{widget,stateful_widget}.rs`：删 `unsafe impl Send/Sync`；`unsafe impl Props` → `impl Props`
- [x] 4.3 `components/router/mod.rs`：删 `Route`/`Routes` 的 `unsafe impl Send/Sync`
- [x] 4.4 `context.rs`：删 `SystemContext` 的 `unsafe impl Send/Sync`

## 5. 保留项确认（不动）

- [x] 5.1 确认 `State`/`StoreState` 仍 `Send + Sync`（`SyncStorage`）——后台 `tokio::spawn` 更新状态的刚需（见决策 6），本变更不动

## 6. 验证

- [x] 6.1 `cargo build --all-features` 逐步收敛报错；`grep -rn "unsafe impl Send\|unsafe impl Sync" packages/ratatui-kit/src` 结果为 **0**
- [x] 6.2 examples 编译；确认 `#[tokio::main]` 无需改（若个别报 Send 错才按需调 `current_thread` 并记录）
- [x] 6.3 四件套全绿（`--all-features`）+ 现有 23 单测 + trybuild 通过；行为与改造前一致
