## Why

ratatui-kit 当前的事件系统是**广播模型**:`Terminal::wait` 把每个 crossterm 事件 clone 后推给所有 `use_events` 订阅者,各 handler 在自己的 `poll_change` 里独立执行。框架**没有**焦点、层级、事件消费(consume）任何原语,因此**输入无法互斥**——打开 `Modal` 后,弹窗背后的组件仍会处理同一个按键(背景列表照常响应方向键/Enter,全局快捷键 `q` 仍退出程序)。

这迫使应用层用 `is_inputting`（全局 bool）、各 `*_modal_open` 标志、`index` 仲裁、`pending_exit`（延迟一帧规避「同一 Enter 被父列表抢跑」的广播竞争）等手段**替框架做事件归属管理**。这些门控散落各处、靠手算布尔组合，新增弹窗极易漏改导致穿透。这是框架级能力缺口，应在框架层根治，而非每个组件 `if !open { return }`。

## What Changes

- **BREAKING**：移除 `Terminal` 的发布订阅机制（`subscribers` / `TerminalEvents` / `events()` / `UpdaterTerminal::events`），`Terminal` 退化为单一 raw event 源（`next_event`）。
- **BREAKING**：移除 `use_events` / `use_local_events` 的广播语义与 hook。事件入口统一为新的可消费模型。
- 引入**中央事件分发**：`InputRuntime`（挂在 `SystemContext`），注册表**每帧随 update 重建**（与 `update_children` 用 `used_components` 重建子树同构），渲染循环在拿到 raw event 后同步 `dispatch`。
- 引入 **InputLayer 栈**：层默认 `blocks_lower`（独占输入），最顶层活跃层之下的组件收不到键盘事件；`EventScope::Global` 可显式穿透。
- 引入**事件消费语义** `EventResult { Consumed, Ignored }` 与 **优先级** `EventPriority`：**层 z-order 优先于优先级**有序投递（更靠栈顶的层整体先，同层内才按优先级），`Consumed` 截断后续 handler——从根上消灭「同一 Enter 被多个 handler 抢」；`Global` handler 独立 phase、不受层截断（Resize / 帮助键）。
- 新增 hooks：`use_input_layer(open, blocks_lower) -> InputLayer` 与 `use_event_handler(scope, priority, FnMut(Event) -> EventResult)`；`scope ∈ { Current（继承 context 注入层）, Layer(handle), Global }`。
- `Modal` 升级：打开时自动注册独占输入层并向子树注入当前层，背景组件天然静默；不再只是绘制遮罩。
- **应用层清理（TRNovel）**：删除 `is_inputting` 全局 bool、`search_input` 的 `pending_exit` hack、以及各页 `!modal_open` / `!info_modal_open` / `current.is_none()` 手算门控；30 处事件监听全量迁移到新模型。

## Capabilities

### New Capabilities

- `input-events`：框架级输入事件分发能力。涵盖单一 raw event 源、中央 `InputRuntime` 分发器（每帧重建注册表）、`InputLayer` 独占栈与 `blocks_lower` 互斥、`EventResult` 消费与 `EventPriority` 有序投递、`EventScope`（层归属/全局穿透）、`use_input_layer` 与 `use_event_handler` 两个 hook，以及 `Modal` 的独占输入层语义。

### Modified Capabilities

<!-- 现有 specs 中无事件系统相关 capability；本变更为纯新增。single-threaded-runtime 的「单线程渲染」前提被新设计复用（事件 handler 闭包免 Send + Sync），但其需求不变，不在此修改。 -->

## Impact

- **框架核心**：`terminal/mod.rs`（删发布订阅、改 `next_event`）、`render/tree.rs`（`render_loop` 改造 + `begin_frame` + `dispatch` 衔接）、`context.rs`（`SystemContext` 挂 `InputRuntime`）、新增 `input_runtime`/`focus` 模块、`render/updater.rs`（`UpdaterTerminal` trait 收缩、`CurrentLayer` 注入）。
- **Hooks**：删除 `hooks/use_events.rs` 的广播实现，新增 `use_input_layer` / `use_event_handler`；鼠标命中过滤迁为 `use_event_handler` 选项。context-aware hook 在**手写 Component**（如 `ScrollView`）中需先 `hooks.with_context_stack(updater.component_context_stack())`（函数组件由 `#[component]` 宏自动处理；不在框架层统一注入因 `Component::update` 同持 hooks 与 `&mut updater` 会借用冲突）。
- **内置组件**：`components/modal.rs` 注册独占层；`input`/`tree` 等门控组件的事件处理迁移。
- **测试与示例**：`render/harness.rs`（`UpdaterTerminal::events` 移除）需同步；新增 `InputRuntime` 纯逻辑单测（`#[cfg(test)]`，不启动终端，覆盖层截断 / z-order 优先 / `Consumed` / `hit_test` / inactive layer 跳过）；新增 `examples/input_mutex.rs` 验证「嵌套 Modal + Modal 内 input + 背景列表 + Layer(h) 三件套配对」互斥与 state→重绘不丢唤醒。
- **应用（TRNovel，下游验证）**：26 文件 30 处 `use_events` 全量迁移，删除 `is_inputting`/`pending_exit`/手动门控。
- **运行时不变量**：保持 `poll_change` 三路全 poll（防丢唤醒）、`State`/`AtomState` 的 `Send + Sync`（`SyncStorage`）、context 查找三态语义；事件 handler 闭包依赖单线程运行时免 `Send + Sync`。
