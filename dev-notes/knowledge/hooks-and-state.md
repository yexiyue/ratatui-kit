# Hooks & State（钩子与状态管理）

## 概览

本主题覆盖 ratatui-kit 的 Hooks 系统约束（**调用顺序必须稳定**）、自定义 Hook 的 `Sealed` trait 标准写法、以及两套状态体系（局部 `use_state` vs 全局 `Atom`）的语义差异与 `Copy`/运算符重载行为。写新 Hook、改 `hooks/`、动 `atom/` 前先读本文件。

## Hooks 系统

### Hook 按调用顺序索引——禁止放进条件/循环

`Hooks` 管理器按**调用顺序**（`hook_index`）索引：首帧 `push`，后续帧按序 `downcast` 取回。因此必须遵守 React 式「Hook 调用顺序每帧稳定」规则。

**正确做法**：所有 `use_*` 调用放在组件函数体顶层，每帧固定顺序执行。

**不要做**：把 `use_state` / `use_future` 等放进 `if` / `for` / `match` 分支或提前 `return` 之后——会让后续帧的 `hook_index` 错位，`downcast` 取到错误类型的 hook（panic 或行为错乱）。

**相关文件**：`crates/ratatui-kit/src/hooks/mod.rs`

### 自定义 Hook：Sealed trait + use_hook 注册的固定四件套

每个内置 hook 都是同一模板，新增 Hook 照抄：

1. `mod private { pub trait Sealed {} impl Sealed for crate::Hooks<'_, '_> {} }` —— 封印，禁止外部为 `Hooks` 实现该扩展 trait。
2. 一个实现 `Hook` trait 的结构体 `UseXxxImpl` 管理状态，按需实现 `poll_change` / `pre|post_component_update` / `pre|post_component_draw` / `on_drop`。
3. `pub trait UseXxx: private::Sealed { fn use_xxx(&mut self, ...) -> ...; }`，仅为 `Hooks` 实现。
4. 方法体内 `let hook = self.use_hook(UseXxxImpl::new);` 注册并取回实例。

若 Hook 持有需驱动重绘的状态，**必须实现 `poll_change`** 并在状态变更时唤醒存好的 `Waker`，否则改了状态画面不会刷新（见 `runtime-architecture.md` 的响应式说明）。

**相关文件**：`crates/ratatui-kit/src/hooks/use_size.rs`（`UseTerminalSize`/`UsePreviousSize` 范例）、`crates/ratatui-kit/src/hooks/mod.rs`（`Hook` trait + `use_hook`）

### 内置 Hook 清单

`use_state` `use_future` `use_event_handler`/`use_input_layer` `use_context` `use_memo` `use_effect` `use_async_state` `use_insert_before` `use_size` `use_exit` `use_on_drop`；特性门控的 `use_router`/`use_navigate`（`router`，同一文件内）、`use_atom`（`atom`）。`use_navigate` 定义在 `use_router.rs` 内，不是单独文件。

**相关文件**：`crates/ratatui-kit/src/hooks/`

### 输入事件系统：use_event_handler 取代了 use_events / use_local_events

旧的「广播订阅」模型（`use_events` / `use_local_events`，所有 handler 平等收到同一事件）已**删除**，换成中央分发器 `InputRuntime`（输入层栈 + 事件消费 + 优先级/作用域），每帧重建。新 API 在 prelude 中。

**迁移映射**（闭包**必须返回** `EventResult`，不消费返回 `EventResult::Ignored`，处理后想截断后续 handler 返回 `EventResult::Consumed`）：

- `hooks.use_events(|e| { BODY })` → `hooks.use_event_handler(EventScope::Current, EventPriority::Normal, |e| { BODY; EventResult::Ignored })`
- `hooks.use_local_events(|e| { BODY })` → `hooks.use_event_handler_with_options(EventScope::Current, EventPriority::Normal, EventOptions { hit_test: true }, |e| { BODY; EventResult::Ignored })`（`hit_test` 复刻旧 local 的鼠标命中过滤）
- 闭包内每个 early `return` 也要返回 `EventResult`（如 `return EventResult::Ignored;`）。

**输入互斥三路径**（canonical demo：`examples/input/input_mutex.rs`）：

- **背景层**（root）：`use_event_handler(EventScope::Current, ..)`，返回 `Ignored`。
- **独占输入层**：`let l = hooks.use_input_layer(open, true);`（`blocks_lower=true` 截断更低层）+ `use_event_handler(EventScope::Layer(l), EventPriority::High, ..)`，处理后返回 `Consumed`。`open=false` 时层不入栈，绑定它的 handler 静默跳过。
- **Modal 层(h) footgun**：`use_input_layer` 拿到的句柄必须**同时**传给 `use_event_handler(EventScope::Layer(h), ..)` 和 `Modal(layer: Some(h))`。漏传 `Modal(layer:)` → Modal 自开新层截断 `h` → 父级 handler 失聪。

**关键约束**：
- `InputLayer` 句柄**跨帧不稳定**（每帧重铸），**禁止**存入 `use_state`；每帧由 `use_input_layer` 重新获取。
- `use_event_handler`/`use_input_layer` 需在 **context-aware** 的 `Hooks` 上调用：`#[component]` 函数组件由宏自动 `with_context_stack`，开箱即用；**手写 `Component`** 须在 `update` 体内先 `let mut hooks = hooks.with_context_stack(updater.component_context_stack());`。
- z-order 优先于 priority（下层 High 不抢上层 Normal）——见 input 模块单测 `layer_z_order_beats_priority`。
- `use_terminal_size` 是例外：它内部经 `post_component_update` 从 `ComponentUpdater` 拿 `SystemContext` 注册 Global Resize，因此手写 `Component` 仍可直接调用，不需要为了它单独 `with_context_stack`。

**相关文件**：`crates/ratatui-kit/src/input/mod.rs`、`crates/ratatui-kit/src/hooks/use_input.rs`、`crates/ratatui-kit/src/components/modal.rs`、`examples/input/input_mutex.rs`

### 内置输入互斥组件优先复用

`SearchInput`、`ConfirmModal`、`AlertModal` 与 `ShortcutInfoModal` 已把常见的输入互斥模式封装成内置组件：`SearchInput` 用局部编辑态 + 独占输入层替代应用级 `is_inputting` / `pending_exit`，三类弹窗内部配对 `use_input_layer`、`use_event_handler(EventScope::Layer(h))` 与 `Modal(layer: Some(h))`，避免父级 handler 失聪。

**正确做法**：
- 底层单行输入用 `Input`，外部持有 `tui_input::Input` 并在业务 handler 中调用 `handle_event`；`Input` 只负责渲染值、占位符和光标，不自带输入层或提交语义。
- 下游搜索框优先用 `SearchInput`，业务只传 `value`、`validate`、`on_submit`、`on_clear` 和样式 props。
- `SearchInput::is_editing` 是外部“允许编辑”开关，不是旧全局 `is_inputting`；它变为 `false` 时组件会同步退出内部编辑态，避免输入层关闭但 UI 仍显示 active。
- `SearchInput` 可直接接收 `width`、`margin`、`offset`，但高度固定为 3 行，避免单行输入框在不同容器里被拉伸。
- 普通确认弹窗优先用 `ConfirmModal`，业务只处理 `open`、`on_confirm`、`on_cancel`。
- `ConfirmModal` 的 `selected_button_style` 会应用到选中按钮边框和整行按钮内容；带背景色的选中样式会用背景色点亮边框，但不会铺满整个按钮容器，避免终端里出现厚色块或标签贴片感。
- 普通提示弹窗优先用 `AlertModal`，业务只处理 `open`、`message`、`on_close`。
- `AlertModal` 默认关闭键是 `Esc` / `Enter`，`close_hint` 默认居中；非关闭键同样返回 `Consumed`，用于保护背景层。
- 快捷键帮助弹窗优先用 `ShortcutInfoModal`；它只消费关闭键，其它键会留给内部 `ScrollView`，背景层仍由输入层截断。
- 只有弹窗内有专用复杂交互时，再手写 `Layer(h)` 三件套。

**相关文件**：`crates/ratatui-kit/src/components/search_input.rs`、`crates/ratatui-kit/src/components/confirm_modal.rs`、`crates/ratatui-kit/src/components/alert_modal.rs`、`crates/ratatui-kit/src/components/shortcut_info_modal.rs`、`examples/components/search_input.rs`、`examples/components/confirm_modal.rs`、`examples/components/alert_modal.rs`、`examples/components/shortcut_info_modal.rs`

### TRNovel 通用选择组件迁移边界

`Select`、`MultiSelect` 与 `TreeSelect` 已把 TRNovel 中无业务依赖的选择模式沉为内置组件：前两者复用 ratatui 原生 `List`/`ListState`，后者复用 `tui-tree-widget`。这些组件都可在内部注册 `Current` 层键盘 handler，`active=false` 时不处理事件；命中自身导航/确认键返回 `Consumed`，其它键返回 `Ignored` 交给同层或父级继续处理。

**正确做法**：
- 简单单选优先用 `Select`，业务传 `items`、`on_select`、标题、样式和可选外部 `state`。
- 普通多选优先用 `MultiSelect`，业务传 `items`、`on_change`、`on_select` 和可选外部 `selected: State<HashSet<usize>>`。
- `Select` / `MultiSelect` 可直接接收 `width`、`height`、`margin`、`offset` 这类外层布局字段，并转发给内部 `Border`；不需要只为定宽再包一层 `View`。
- `Select` / `MultiSelect` 会在 `items` 变短时把列表游标夹回有效范围；`MultiSelect` 还会裁掉越界选中项，适合搜索过滤后的动态列表。空列表只渲染 empty 状态，不消费导航/确认键。
- `Select` / `MultiSelect` / `VirtualList` 的 `default_index` 支持先空后加载：空列表不会永久吞掉默认游标，数据回来且当前无游标时会补选默认项；用户已有游标时不会因列表长度变化被强制重置。`MultiSelect` 的 `default_index` 只设置光标，不会自动勾选项目。
- 树形选择优先用 `TreeSelect`（`tree` feature），业务传 `TreeItem<T>`、`on_select` 和样式 props；如果只是渲染树，保持默认 `active=false`。
- `TreeSelect` 的 `default_selection` 是节点路径（如 `["components", "select"]`），不是索引；组件会选中末端节点并打开祖先节点。它可直接接收 `width`、`height`、`margin`、`offset`，边框仍使用 `tui-tree-widget` 原生 `Block`。
- `Select` / `MultiSelect` / `TreeSelect` 适合放在 Modal 子树内；它们的 `Current` handler 会自动归属 Modal 注入的层。
- 长列表/自定义 item 渲染优先用 `VirtualList`（`virtual-list` feature），业务传 `item_count`、`render_item`、可选外部 `tui_widget_list::ListState` 和 `on_select(index)`；empty/loading 外壳仍由业务组合。`VirtualList` 可直接接收 `width`、`height`、`margin`、`offset`，边框仍使用 `tui-widget-list` 原生 `Block`。
- TRNovel 的 `ListSelect` / `MultiListSelect` 是 `VirtualList` + empty/loading + 业务主题的组合；框架只内置低层虚拟列表，不把业务 loading 文案或主题依赖混入通用选择组件。虚拟多选可参考 `examples/components/virtual_multi_select.rs`：外部持有 `ListState` 和 `HashSet<usize>`，Space/Enter 由父级业务 handler 处理，导航仍交给 `VirtualList`。

**相关文件**：`crates/ratatui-kit/src/components/select.rs`、`crates/ratatui-kit/src/components/multi_select.rs`、`crates/ratatui-kit/src/components/tree_select.rs`、`crates/ratatui-kit/src/components/virtual_list.rs`、`examples/components/select.rs`、`examples/components/multi_select.rs`、`examples/components/tree_select.rs`、`examples/components/virtual_list.rs`、`examples/components/virtual_multi_select.rs`

### TRNovel 剩余组件沉淀裁决

TRNovel 是当前内置组件抽取样本，但框架 API 不能带业务名或业务依赖。迁移时按下表处理：

| TRNovel 组件 | 框架侧归宿 | 裁决 |
|---|---|---|
| `search_input.rs` | `SearchInput` | 已内置，替代全局 `is_inputting` / `pending_exit`。 |
| `modal/confirm.rs` | `ConfirmModal` | 已内置，业务只接 `open`、`on_confirm`、`on_cancel`。 |
| `modal/warning.rs` | `AlertModal` | 已内置普通提示；错误态退出由业务在 `on_close` 中决定，不新增 `ErrorModal`。 |
| `modal/shortcut_info_modal.rs` | `ShortcutInfoModal` | 已内置，关闭键在组件内消费，滚动键留给内部 `ScrollView`。 |
| `select.rs` | `Select` | 已内置普通单选。 |
| `multi_list_select.rs` | `MultiSelect` + `VirtualList` | 普通多选已内置；虚拟多选可由 `VirtualList` + 业务选中集合组合。 |
| TRNovel 文件树选择用法 | `TreeSelect` | 只沉淀通用树形选择；文件语义留给业务。 |
| `list_view.rs` / `list_select.rs` | `VirtualList` | 低层虚拟列表已内置在 `virtual-list` feature；`ListSelect` 的 empty/loading/主题包装留给业务。 |
| `read_novel/read_content.rs` 的正文换行 | `WrappedText` + `ScrollView` | 长纯文本先按宽度硬换行并把真实行数交给布局；阅读页、日志、帮助文档都可复用。 |
| `loading.rs` | 暂不内置 | 依赖 `throbber-widgets-tui` + tokio 定时器；若框架要提供加载态，应另设 spinner/loading feature。 |
| `modal/browser_prompt.rs` | 不内置 | 绑定 `browser_assist` / 书源验证业务，可由 `AlertModal` / `ConfirmModal` 组合实现。 |

**正确做法**：
- 先迁移到已有内置组件，再评估是否还缺少通用能力。
- 引入新依赖前先做 feature 设计；不要把 TRNovel 的业务依赖带进核心默认组件。
- 组件命名保持领域中立；树形选择统一叫 `TreeSelect`。
- 长正文换行保持领域中立；组件叫 `WrappedText`，不要叫 `NovelText` 或绑定阅读进度/TTS。

**相关文件**：`../TRNovel/src/components/`、`../TRNovel/src/pages/read_novel/read_content.rs`、`crates/ratatui-kit/src/components/`

### WrappedText 承接长正文硬换行

TRNovel 的阅读页不是单纯依赖 ratatui `Paragraph::wrap`，而是先用 `textwrap::fill(content, width)` 把正文按终端宽度展开成真实多行，再用行数驱动滚动进度。框架侧沉淀为 `WrappedText`：它接收纯文本、`wrap_width`、整体样式和滚动偏移，用 `textwrap` 预换行，并默认把自身高度设为换行后的行数。

**正确做法**：
- 普通短文本/富文本继续用 `Text`；只需要绘制时软换行可用 `Text(wrap: true)`。
- 长纯文本放进 `ScrollView` 时优先用 `WrappedText(text, wrap_width)`，让 ScrollView 能拿到真实内容高度。
- `wrap_width` 建议由调用方按容器内宽显式传入；当前布局系统在 update 阶段拿不到最终 Fill 宽度，不能完全自动推导响应式宽度。
- `WrappedText` 保持纯文本组件，不混入小说阅读进度、章节切换、TTS 高亮等业务能力。

**相关文件**：`crates/ratatui-kit/src/components/wrapped_text.rs`、`examples/components/wrapped_text.rs`、`../TRNovel/src/pages/read_novel/read_content.rs`

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

**相关文件**：`crates/ratatui-kit/src/hooks/use_state.rs`、`crates/ratatui-kit/src/atom/mod.rs`、`crates/ratatui-kit/src/atom/use_atom.rs`

### State/AtomState 重载了算术运算符——`+=` 等会触发变更通知

`State<T>` 对 `T: Copy` 实现了 `Add/Sub/Mul/Div` 及对应 `*Assign`（`AtomState` 同理）。`count += 1` 这类写法既更新值又唤醒 Waker 触发重绘。两者的运算符实现由 `reactive_ops.rs` 的同一个宏生成。

**正确做法**：用 `state += 1` / `state.set(v)` / `state.write()` 修改，让变更走唤醒通道。读用 `state.get()`（`Copy` 值）或 `state.read()`（借用）。

**不要做**：绕过句柄方法直接操作底层 `generational-box` 存储——会跳过 Waker 唤醒，画面不刷新。

**相关文件**：`crates/ratatui-kit/src/hooks/use_state.rs`、`crates/ratatui-kit/src/atom/mod.rs`、`crates/ratatui-kit/src/reactive_ops.rs`

### use_atom 会跟随传入的 Atom，并负责退订

`Hooks::use_hook` 只在首帧运行初始化闭包，因此带外部参数的 Hook 需要在每帧主动同步参数。`use_atom(&ATOM)` 每帧都会把 hook 内部句柄校准到当前传入的 atom；当 atom 改变或组件卸载时，会移除旧 atom 上以组件 key 注册的 waker，避免旧 atom 继续唤醒已切走/已卸载的组件。

**正确做法**：自定义 Hook 若依赖 props/参数，不要只把参数写进 `use_hook(|| ...)` 的初始化闭包；后续帧也要更新 hook 内部状态，并清理旧订阅/资源。

**相关文件**：`crates/ratatui-kit/src/atom/use_atom.rs`

### ReactiveHandle 是 State/AtomState 的单一真源

`State<T>` 与 `AtomState<T>` 的读写访问、`try_*`、格式化/比较/Hash 和算术运算符由 `ReactiveHandle<T, N>` 统一提供；差异只在 notifier：`SingleWaker`（本地 state）和 `WakerMap`（按组件 key 订阅的 atom）。

**正确做法**：
- 修改响应式读写语义时优先改 `reactive_handle.rs`，不要在 `use_state.rs` 和 `atom/mod.rs` 复制两份实现。
- `State`/`AtomState` 继续使用 `SyncStorage`，并保持 `Send + Sync`，以支持后台任务持有句柄写入并唤醒 UI。

**不要做**：
- 不要恢复 `try_read` 中的 `loop`/`continue` 重试；`try_*` 获取不到借用时应直接返回 `None`，`read()`/`write()` 由 `expect` 快速暴露编程错误。
- 不要把本地 State 改回 `UnsyncStorage`。

**相关文件**：`crates/ratatui-kit/src/reactive_handle.rs`、`crates/ratatui-kit/src/hooks/use_state.rs`、`crates/ratatui-kit/src/atom/mod.rs`

### 依赖型 Hook 用 PartialEq 比较依赖

`use_memo`、`use_effect`、`use_async_effect` 的依赖从 `Hash` 改为 `PartialEq`，hook 内保存 `Option<D>`：`None` 表示首帧未运行，保证首次必跑；后续用值相等判断是否重算/重跑，避免哈希碰撞漏更新。依赖值在变化时直接移动进 hook 保存，不再要求 `Clone`。

**正确做法**：
- deps 选小而稳定的 `PartialEq` 值或元组。
- 自定义依赖型 Hook 若需要首次必跑，优先存 `Option<D>`，不要用 `0` 这类哨兵值。
- `use_effect` 维护独立的依赖状态，不再复用 `use_memo`；effect 和 memo 的语义在代码和文档中分开讲。

**相关文件**：`crates/ratatui-kit/src/hooks/use_memo.rs`、`crates/ratatui-kit/src/hooks/use_effect.rs`、`crates/ratatui-kit/src/hooks/use_future.rs`

### 组合型 Hook 可直接复用内置 Hook

不是所有内置 hook 都需要额外 `use_hook` 一个专属状态结构。像 `use_async_state` 这种纯组合型 hook，本质是 `use_state` + `use_async_effect` 的稳定调用序列，直接复用已有 hooks 更简单，也避免 hook 列表里出现无状态占位。

**正确做法**：
- 组合型 hook 内部的 `use_*` 调用顺序仍必须每帧稳定。
- 只有 hook 自己持有无法由现有 hooks 表达的状态或生命周期时，才新增 `UseXxxImpl` 并实现 `Hook`。

**相关文件**：`crates/ratatui-kit/src/hooks/use_async_state.rs`

### use_async_state 沉淀 TRNovel 的异步数据三态

TRNovel 多处页面把异步请求拆成 `data/loading/error` 三个 `State`，并通过依赖控制请求重跑。框架侧抽象为通用 `use_async_state(f, deps)`：依赖变化时取消旧 future、运行新 future，立即置 `loading=true`，成功写 `data`，失败写 `error`，完成后置 `loading=false`。刷新期间默认保留旧 `data`，适合列表、详情、搜索结果这类“旧内容可继续显示”的终端界面。

**正确做法**：
- 把真实异步工作放进传给 `use_async_state` 的 future 工厂里，由 deps 控制何时启动。
- 失败态只清/写 `error`，不默认清空旧 `data`；需要空态或骨架屏时由业务组件决定。
- 不在框架核心加入 tokio timer、debounce 或业务 loading 文案；这些可在应用层组合，或以后设计独立 feature。

**不要做**：
- 不要在组件函数体顶层直接 `tokio::spawn` 请求再写 state；这会绕过 deps，每帧都可能启动新任务。

**相关文件**：`crates/ratatui-kit/src/hooks/use_async_state.rs`、`examples/hooks/async_state.rs`、`../TRNovel/src/hooks/use_init_state.rs`

### Router state 只由 with_state 导航显式携带

`RouteState` 是单个 history entry 的可选 payload，适合列表进入详情时携带来源提示或轻量上下文。普通 `navigate.push(path)` / `navigate.replace(path)` 必须清空当前 `RouteState`；只有 `push_with_state` / `replace_with_state` 会携带新的 state。

**正确做法**：
- URL 必要信息放进 path params，例如 `/projects/:slug`。
- 一次性、非 URL 语义的上下文用 `push_with_state` 显式传入。
- `try_use_route_state::<T>()` 用于可选读取；断言型 `use_route_state::<T>()` 只在页面没有该 state 就是编程错误时使用。

**不要做**：
- 不要让普通 `push` / `replace` 从当前页面隐式继承 state；这会让旧详情页 state 泄漏到无关页面。
- 不要把 Router 默认状态放进 Atom。Router 是 `RouterProvider` 作用域状态，Atom 是进程级状态。

**相关文件**：`crates/ratatui-kit/src/hooks/use_router.rs`、`examples/routing/router.rs`、`docs/src/content/docs/core/routing.mdx`

### Router history_length 最小为 1

`RouterProvider(history_length: Some(n))` 的有效最小值是 1。`0` 会在 `RouterHistory::new` 入口被 clamp 成 1，语义是只保留当前最新 entry；`RouterHistory::push` 也会防御性 normalize 内部 `max_length`，避免测试或内部构造绕过入口后留下非法状态。

**正确做法**：
- 业务想限制 history 时传正整数；只保留当前页用 `history_length: 1`。
- 内部构造 `RouterHistory` 优先用 `RouterHistory::new(initial_context, max_length)`，不要直接手写字段。

**相关文件**：`crates/ratatui-kit/src/components/router/history.rs`、`crates/ratatui-kit/src/components/router/router_provider.rs`、`docs/src/content/docs/core/routing.mdx`

### 主题系统:Palette 唯一色源 + per-component ComponentTheme

主题协议 always-on(零新依赖,`crates/ratatui-kit/src/components/theme/`)。`Palette` 是**唯一颜色真源**(`#[non_exhaustive]`,`Palette::default()` 后改字段构造);每个组件定义自己的 `FooTheme`(`#[non_exhaustive]` + `Clone` + `Default` + `impl ComponentTheme`;内置主题同时 `Copy`;`Default == from_palette(&Palette::default())`),`from_palette(&Palette)` 只从 palette 取色、`DIM`/`BOLD`/可见选中态等非颜色决定在此承接。读取两条路径:函数组件用 `hooks.use_component_theme::<T>()`(`UseTheme`,Sealed);手写 `Component` 用 `updater.use_component_theme::<T>()`(`ComponentUpdater` 的 inherent 方法)。二者都返回 **owned 值**(内部 `try_use_context`/`get_context` 后 clone、读后即弃守卫)。解析链:显式 `FooTheme` override context → `from_palette(&palette)` → `default()`。

样式 props 一律 `Option<Style>`,应用统一经 `crate::components::theme::resolve_style(theme_slot, prop_override)`，语义等价于 `theme.slot.patch(prop.unwrap_or_default())`——`None` 用主题,`Some(s)` 以 `patch` 覆盖,`Some(Style::reset())` 清回终端默认。合成方向承重:`theme.patch(props)`(props 的 Some 字段胜出)。

**正确做法**:
- 手写组件读主题:先跑完 `hooks`(其 `with_context_stack` 借了 `updater`),再 `updater.use_component_theme::<T>()`;NLL 释放后二者不冲突。样式字段在 `new`/`from_props` 里置 `Style::default()` 占位,`update` 里经主题解析后写入(Border/Modal/TreeSelect/VirtualList/Table 均此法);`Table` 的估高路径与样式无关,占位样式不影响。
- 运行时换肤:把 `Palette` 放进 `Atom<Palette>` / `use_state`,`use_atom(&PALETTE)` 订阅 + `PaletteProvider(palette: ...)` 注入;写 Atom 唤醒整树重渲换色(见 `examples/components/theme.rs` 与 `render/harness.rs` 的 `runtime_theme_tests`)。
- 组合组件(SearchInput/ConfirmModal/三 modal)自解析 `FooTheme` 后把 resolved `Style` 透传给内层 Border/Text/Input;内层组件如实渲染,不产生双重上色。modal 遮罩(DIM)委托给 `Modal` 的 `ModalTheme`,组合组件的 `style` prop 默认 `None` 透传即可。

**不要做**:
- 不要指望 context 自动响应式:`use_palette`/`use_component_theme` 是**被动读取**(不注册 hook/waker),换肤必须靠 Atom/state 驱动 Provider 重渲,不是 context 自己变。
- 不要在手写组件里持有 `Ref` 守卫的同时 `update_children` / 拿 `&mut updater`——会 `AlreadyBorrowed` panic 或借用冲突;`use_*_theme` 已帮你 clone+drop,直接用返回的 owned 值。
- 不要给中性组件的 slot 硬塞可见色而破坏原语义,除非是刻意的默认改进(如 `TreeSelectTheme` 补默认可见选中、`ModalTheme` 默认 DIM 遮罩——均可被 `Style::reset()` 清掉)。

**相关文件**:`crates/ratatui-kit/src/components/theme/{mod.rs,palette.rs}`、各组件的 `FooTheme`(如 `components/border.rs`、`components/table/component.rs`)、`examples/components/theme.rs`、`render/harness.rs`(`theme_tests`/`runtime_theme_tests`)、`EXTENSION_API.md`
