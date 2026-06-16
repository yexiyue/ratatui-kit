# Hooks & State（钩子与状态管理）

## 概览

本主题覆盖 ratatui-kit 的 Hooks 系统约束（**调用顺序必须稳定**）、自定义 Hook 的 `Sealed` trait 标准写法、以及两套状态体系（局部 `use_state` vs 全局 `Atom`）的语义差异与 `Copy`/运算符重载行为。写新 Hook、改 `hooks/`、动 `atom/` 前先读本文件。

## Hooks 系统

### Hook 按调用顺序索引——禁止放进条件/循环

`Hooks` 管理器按**调用顺序**（`hook_index`）索引：首帧 `push`，后续帧按序 `downcast` 取回。因此必须遵守 React 式「Hook 调用顺序每帧稳定」规则。

**正确做法**：所有 `use_*` 调用放在组件函数体顶层，每帧固定顺序执行。

**不要做**：把 `use_state` / `use_future` 等放进 `if` / `for` / `match` 分支或提前 `return` 之后——会让后续帧的 `hook_index` 错位，`downcast` 取到错误类型的 hook（panic 或行为错乱）。

**相关文件**：`packages/ratatui-kit/src/hooks/mod.rs`

### 自定义 Hook：Sealed trait + use_hook 注册的固定四件套

每个内置 hook 都是同一模板，新增 Hook 照抄：

1. `mod private { pub trait Sealed {} impl Sealed for crate::Hooks<'_, '_> {} }` —— 封印，禁止外部为 `Hooks` 实现该扩展 trait。
2. 一个实现 `Hook` trait 的结构体 `UseXxxImpl` 管理状态，按需实现 `poll_change` / `pre|post_component_update` / `pre|post_component_draw` / `on_drop`。
3. `pub trait UseXxx: private::Sealed { fn use_xxx(&mut self, ...) -> ...; }`，仅为 `Hooks` 实现。
4. 方法体内 `let hook = self.use_hook(UseXxxImpl::new);` 注册并取回实例。

若 Hook 持有需驱动重绘的状态，**必须实现 `poll_change`** 并在状态变更时唤醒存好的 `Waker`，否则改了状态画面不会刷新（见 `runtime-architecture.md` 的响应式说明）。

**相关文件**：`packages/ratatui-kit/src/hooks/use_size.rs`（`UseTerminalSize`/`UsePreviousSize` 范例）、`packages/ratatui-kit/src/hooks/mod.rs`（`Hook` trait + `use_hook`）

### 内置 Hook 清单

`use_state` `use_future` `use_event_handler`/`use_input_layer` `use_context` `use_memo` `use_effect` `use_insert_before` `use_size` `use_exit` `use_on_drop`；特性门控的 `use_router`/`use_navigate`（`router`，同一文件内）、`use_atom`（`atom`）。`use_navigate` 定义在 `use_router.rs` 内，不是单独文件。

**相关文件**：`packages/ratatui-kit/src/hooks/`

### 输入事件系统：use_event_handler 取代了 use_events / use_local_events

旧的「广播订阅」模型（`use_events` / `use_local_events`，所有 handler 平等收到同一事件）已**删除**，换成中央分发器 `InputRuntime`（输入层栈 + 事件消费 + 优先级/作用域），每帧重建。新 API 在 prelude 中。

**迁移映射**（闭包**必须返回** `EventResult`，不消费返回 `EventResult::Ignored`，处理后想截断后续 handler 返回 `EventResult::Consumed`）：

- `hooks.use_events(|e| { BODY })` → `hooks.use_event_handler(EventScope::Current, EventPriority::Normal, |e| { BODY; EventResult::Ignored })`
- `hooks.use_local_events(|e| { BODY })` → `hooks.use_event_handler_with_options(EventScope::Current, EventPriority::Normal, EventOptions { hit_test: true }, |e| { BODY; EventResult::Ignored })`（`hit_test` 复刻旧 local 的鼠标命中过滤）
- 闭包内每个 early `return` 也要返回 `EventResult`（如 `return EventResult::Ignored;`）。

**输入互斥三路径**（canonical demo：`examples/input_mutex.rs`）：

- **背景层**（root）：`use_event_handler(EventScope::Current, ..)`，返回 `Ignored`。
- **独占输入层**：`let l = hooks.use_input_layer(open, true);`（`blocks_lower=true` 截断更低层）+ `use_event_handler(EventScope::Layer(l), EventPriority::High, ..)`，处理后返回 `Consumed`。`open=false` 时层不入栈，绑定它的 handler 静默跳过。
- **Modal 层(h) footgun**：`use_input_layer` 拿到的句柄必须**同时**传给 `use_event_handler(EventScope::Layer(h), ..)` 和 `Modal(layer: Some(h))`。漏传 `Modal(layer:)` → Modal 自开新层截断 `h` → 父级 handler 失聪。

**关键约束**：
- `InputLayer` 句柄**跨帧不稳定**（每帧重铸），**禁止**存入 `use_state`；每帧由 `use_input_layer` 重新获取。
- `use_event_handler`/`use_input_layer` 需在 **context-aware** 的 `Hooks` 上调用：`#[component]` 函数组件由宏自动 `with_context_stack`，开箱即用；**手写 `Component`** 须在 `update` 体内先 `let mut hooks = hooks.with_context_stack(updater.component_context_stack());`。
- z-order 优先于 priority（下层 High 不抢上层 Normal）——见 input 模块单测 `layer_z_order_beats_priority`。
- `use_terminal_size` 是例外：它内部经 `post_component_update` 从 `ComponentUpdater` 拿 `SystemContext` 注册 Global Resize，因此手写 `Component` 仍可直接调用，不需要为了它单独 `with_context_stack`。

**相关文件**：`packages/ratatui-kit/src/input/mod.rs`、`packages/ratatui-kit/src/hooks/use_input.rs`、`packages/ratatui-kit/src/components/modal.rs`、`examples/input_mutex.rs`

## 两套状态体系

### 局部 use_state vs 全局 Atom：生命周期与作用域不同

| | `use_state` | Atom（`atom` feature） |
|---|---|---|
| Owner | 每组件独立 `Owner`，随组件卸载释放 | 进程级 `LazyLock<Owner>`，全程存活 |
| 句柄类型 | `State<T>` | `AtomState<T>` |
| 声明方式 | `let s = hooks.use_state(\|\| 0);` | `static COUNT: Atom<i32> = Atom::new(|| 0);` |
| 订阅方式 | 直接持有 | `hooks.use_atom(&COUNT)` 订阅单个原子 |

两者底层都是 `generational-box`，句柄都实现了 `Copy`（可随意按值传进闭包/子组件，无需 clone）。`Atom<T>` 本身是全局声明入口，`AtomState<T>` 是读写句柄。

**正确做法**：组件私有、随卸载消失的状态用 `use_state`；跨组件/进程级共享用 `Atom`。`atom` 模块在 `atom` feature 后。

**相关文件**：`packages/ratatui-kit/src/hooks/use_state.rs`、`packages/ratatui-kit/src/atom/mod.rs`、`packages/ratatui-kit/src/atom/use_atom.rs`

### State/AtomState 重载了算术运算符——`+=` 等会触发变更通知

`State<T>` 对 `T: Copy` 实现了 `Add/Sub/Mul/Div` 及对应 `*Assign`（`AtomState` 同理）。`count += 1` 这类写法既更新值又唤醒 Waker 触发重绘。两者的运算符实现由 `reactive_ops.rs` 的同一个宏生成。

**正确做法**：用 `state += 1` / `state.set(v)` / `state.write()` 修改，让变更走唤醒通道。读用 `state.get()`（`Copy` 值）或 `state.read()`（借用）。

**不要做**：绕过句柄方法直接操作底层 `generational-box` 存储——会跳过 Waker 唤醒，画面不刷新。

**相关文件**：`packages/ratatui-kit/src/hooks/use_state.rs`、`packages/ratatui-kit/src/atom/mod.rs`、`packages/ratatui-kit/src/reactive_ops.rs`

### use_atom 会跟随传入的 Atom，并负责退订

`Hooks::use_hook` 只在首帧运行初始化闭包，因此带外部参数的 Hook 需要在每帧主动同步参数。`use_atom(&ATOM)` 每帧都会把 hook 内部句柄校准到当前传入的 atom；当 atom 改变或组件卸载时，会移除旧 atom 上以组件 key 注册的 waker，避免旧 atom 继续唤醒已切走/已卸载的组件。

**正确做法**：自定义 Hook 若依赖 props/参数，不要只把参数写进 `use_hook(|| ...)` 的初始化闭包；后续帧也要更新 hook 内部状态，并清理旧订阅/资源。

**相关文件**：`packages/ratatui-kit/src/atom/use_atom.rs`

### ReactiveHandle 是 State/AtomState 的单一真源

`State<T>` 与 `AtomState<T>` 的读写访问、`try_*`、格式化/比较/Hash 和算术运算符由 `ReactiveHandle<T, N>` 统一提供；差异只在 notifier：`SingleWaker`（本地 state）和 `WakerMap`（按组件 key 订阅的 atom）。

**正确做法**：
- 修改响应式读写语义时优先改 `reactive_handle.rs`，不要在 `use_state.rs` 和 `atom/mod.rs` 复制两份实现。
- `State`/`AtomState` 继续使用 `SyncStorage`，并保持 `Send + Sync`，以支持后台任务持有句柄写入并唤醒 UI。

**不要做**：
- 不要恢复 `try_read` 中的 `loop`/`continue` 重试；`try_*` 获取不到借用时应直接返回 `None`，`read()`/`write()` 由 `expect` 快速暴露编程错误。
- 不要把本地 State 改回 `UnsyncStorage`。

**相关文件**：`packages/ratatui-kit/src/reactive_handle.rs`、`packages/ratatui-kit/src/hooks/use_state.rs`、`packages/ratatui-kit/src/atom/mod.rs`

### 依赖型 Hook 用 PartialEq 比较依赖

`use_memo`、`use_effect`、`use_async_effect` 的依赖从 `Hash` 改为 `PartialEq + Clone`，hook 内保存 `Option<D>`：`None` 表示首帧未运行，保证首次必跑；后续用值相等判断是否重算/重跑，避免哈希碰撞漏更新。

**正确做法**：
- deps 选小而稳定的 `Copy`/`Clone` 值或元组。
- 自定义依赖型 Hook 若需要首次必跑，优先存 `Option<D>`，不要用 `0` 这类哨兵值。

**相关文件**：`packages/ratatui-kit/src/hooks/use_memo.rs`、`packages/ratatui-kit/src/hooks/use_effect.rs`、`packages/ratatui-kit/src/hooks/use_future.rs`
