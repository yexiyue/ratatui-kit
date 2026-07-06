# Runtime Architecture（声明 → 实例 → 渲染循环）

## 概览

本主题覆盖 ratatui-kit 运行时的核心机制：`Element`（声明）vs `Component`（实例）的二分、协调（reconciliation）的 key 复用语义、`render_loop` 的「状态变更经 Waker 唤醒」响应式模型、以及布局/透明布局。**改 `render/`、`component/`、`element/` 任一模块前先读本文件**——这些不变量一旦破坏，整棵树的状态保持或重绘触发会静默失效。

## 声明 vs 实例

### Element 是声明，InstantiatedComponent 才是状态住所

- `Element<T>` = `{ key, props }`，**每帧重建**的轻量声明，`element!` 宏产出，`AnyElement` 是类型擦除版。
- `InstantiatedComponent` 是**持久化树节点**，持有 component 实例、hooks 列表、子 `Components`、`LayoutStyle`——状态（hooks、use_state）真正存活在这里，跨帧保留。

**正确做法**：理解「Element 廉价、可随意重建；Instantiated 昂贵、靠协调复用」。给组件的副作用/状态都挂在 hooks 上，而不是塞进 props 或 element。

**相关文件**：`crates/ratatui-kit/src/element/mod.rs`、`crates/ratatui-kit/src/component/instantiated_component.rs`

### 协调按 `ElementKey + TypeId` 复用——key 决定状态去留

`ComponentUpdater::update_children` 按 `ElementKey` + 组件 `TypeId` 匹配上一帧节点：命中则复用（保留 hooks/状态），否则新建。这就是「同一 key 同一类型 → 状态保留」的来源，等同 React 的 key diff。

**正确做法**：列表渲染时给每个子元素稳定且唯一的 `key`，否则增删项会错位复用别人的状态。换组件类型（即使 key 相同）会强制重建。

**不要做**：用数组下标当 key 渲染会增删的列表——会导致状态串台。

**相关文件**：`crates/ratatui-kit/src/render/updater.rs`、`crates/ratatui-kit/src/element/key.rs`

## 渲染循环与响应式

### UI 不是命令式重绘，而是状态写入 → Waker 唤醒 → 重渲染

`render/tree.rs` 的 `render_loop` 骨架：

```text
loop {
  render();
  if should_exit break;
  select(component.wait(), terminal.next_event()).await;
  if event { ctrl_c ? break : input.dispatch(event); continue; }
}
```

`render()` 先自顶向下 `update`（跑组件函数体、跑 hooks、协调子树），再 `terminal.draw` 自顶向下 `draw`。然后 `select` 在「组件树有变化」与「终端有事件」之间阻塞，任一就绪即重渲染。

「组件树有变化」由 `poll_change` 聚合：组件 / 子节点 / hooks 三路任一 `Ready` 即唤醒。响应式状态（`use_state` 的 `State<T>`、全局 `AtomState<T>`，均基于 `generational-box`）在写入时唤醒存好的 `Waker`，打破 `select` 阻塞触发下一帧。

**正确做法**：要让 UI 响应某个变化，必须让它经过一个会唤醒 Waker 的通道——`State`/`AtomState` 写入、`use_future` 完成、终端事件。自定义 Hook 若持有需驱动重绘的状态，要实现 `poll_change` 并在变更时唤醒 waker（参见 `hooks-and-state.md`）。

**不要做**：在组件外用普通变量/`static mut` 存 UI 状态再期望它自动重绘——没有 Waker 唤醒，`select` 不会醒，画面卡住直到下一个无关事件。

**相关文件**：`crates/ratatui-kit/src/render/tree.rs`、`crates/ratatui-kit/src/component/mod.rs`（`poll_change`）

## 布局

### LayoutStyle 直接映射 ratatui Layout/Constraint

`LayoutStyle` = `flex_direction / justify_content / gap / margin / offset / width / height`，直译为 ratatui 的 `Layout`/`Constraint`。`Component::calc_children_areas` 默认按 flex 切分子区域，可重写实现自定义布局。

**正确做法**：写需要非 flex 排布的组件（如 `ScrollView`、`Modal`）时重写 `calc_children_areas`，参考现有两者的实现。组件想获得布局字段，用 `#[with_layout_style]` 给 Props 注入（见 `macros-and-props.md`）。

**相关文件**：`crates/ratatui-kit/src/render/layout_style.rs`、`crates/ratatui-kit/src/components/scroll_view/mod.rs`、`crates/ratatui-kit/src/components/modal.rs`

### 透明布局：函数组件是「透传包装器」，布局属性写在子根元素上

`#[component]` 宏生成的函数组件会调用 `set_transparent_layout(true)`，使其**不占独立布局节点**、直接继承首个子组件的 `LayoutStyle`。

**正确做法**：给一个 `#[component]` 函数组件设 width/height/flex 等布局属性时，要把它们写在该函数 **返回的根元素** 上，而不是期望在父级 `element!` 里给这个函数组件传布局 prop 生效。

**不要做**：在 `element!` 里给一个函数组件包装器直接挂布局属性并指望它形成独立布局区——它是透明的，属性会被忽略/穿透。

**相关文件**：`crates/ratatui-kit-macros/src/component.rs`（`set_transparent_layout`）、`crates/ratatui-kit/src/components/view.rs`

### 组件树运行时契约

透明布局组件如果本帧没有子节点，`layout_style` 必须重置为 `LayoutStyle::default()`，不能沿用上一帧从子节点继承的旧布局。`Component::calc_children_areas` 的返回区域数必须等于子节点数；draw 路径会在 debug 下断言这个契约，避免 `zip` 静默丢子节点。

**正确做法**：
- 自定义 `calc_children_areas` 时始终按 children 数量返回区域。
- 条件渲染可能返回空子树的透明组件不需要手动清布局，运行时会重置。

**相关文件**：`crates/ratatui-kit/src/component/mod.rs`、`crates/ratatui-kit/src/component/instantiated_component.rs`

### poll_change 必须三路全 poll

`InstantiatedComponent::poll_change` 会对组件自身、子节点、hooks 三路全部求值。不要把它改成 `||` 短路形式；即使某一路已经 `Ready`，其余 `Pending` 路也需要在本轮注册 waker，否则后续变化可能不会唤醒渲染循环。

**正确做法**：改 poll 聚合逻辑时先保存三路结果，再统一判断是否有 `Ready`。

**相关文件**：`crates/ratatui-kit/src/component/instantiated_component.rs`

### ScrollView 内容尺寸与滚动条判定共用公式

`ScrollView` 的子区域计算和滚动条渲染都通过 `ScrollBars::layout_for` 判断是否预留滚动条并缩小可见区域。尺寸推导中的 `Fill`/`Percentage`/`Ratio` 使用高位宽计算并饱和到 `u16`，负 `gap` 会饱和为 0，避免 debug 溢出或 release 回绕。

**正确做法**：改 ScrollView 显隐或尺寸公式时同时走 `layout_for`，不要在 `calc_children_areas` 和 `render_scrollbars` 分叉维护两套 ±1 规则。

**相关文件**：`crates/ratatui-kit/src/components/scroll_view/mod.rs`、`crates/ratatui-kit/src/components/scroll_view/scrollbars.rs`

### Context 查找区分「已借用」与「未找到」三态

`ContextStack::get_context(_mut)` 返回三态 `ContextLookup`（`Found` / `AlreadyBorrowed` / `NotFound`），而非 `Option`。断言型 `use_context(_mut)` 据此分别给「已被借用」（持守卫重入）与「未找到」（Provider 未注入）两种精确 panic 诊断；`try_use_context(_mut)` 与 `ComponentUpdater::get_context` 则把非 `Found` 一律降级为 `None`，**保持 try_/Option 接口永不 panic**。

`Context` 构造入口按所有权语义命名：`Context::owned(value)`、`Context::from_ref(&value)`、`Context::from_mut(&mut value)`。不要再写旧拼写 `form_ref` / `form_mut`。

**正确做法**：改 context 查找逻辑时保留三态——断言型给诊断、try_/Option 型安全降级。这与响应式状态的 `try_*` 约定一致（见 `hooks-and-state.md`）：`try_` 系列绝不 panic，只有断言型（`use_*` / `read` / `write`）才 panic。

**不要做**：把「已借用」的 panic 放进 `get_context` 这种被 `try_use_context` 复用的共享方法——会让 try_ 变体跟着 panic，破坏其非 panic 契约。

**相关文件**：`crates/ratatui-kit/src/context.rs`、`crates/ratatui-kit/src/hooks/use_context.rs`、`crates/ratatui-kit/src/render/updater.rs`

## 输入事件分发

### 中央 InputRuntime 分发取代广播订阅

事件系统从「广播订阅」(每个 `use_events` 各自订阅、所有 handler 平等收到同一事件)重写为「单 raw 源 + 中央 `InputRuntime` 分发」(`input/mod.rs`)。`Terminal` 退化为纯 raw source（`next_event`,删 `events()`/`wait()`/订阅者)；`render_loop`（`tree.rs`)取一个事件 → `CrossTerminal::received_ctrl_c` 先判退出 → 否则 `system_context.input.dispatch(event)`。

**正确做法**：理解三个不变量——
- **每帧重建**：`update_once` 开头 `input.begin_frame()` 清空层/handler 并铸造 root 层,组件 update 期间经 `use_input_layer`/`use_event_handler` 重新登记。无跨帧持久状态 → 关闭的弹窗/卸载的组件下一帧自动退出,无泄漏、无 id 串号。`begin_frame` 必须在 `ContextStack::root` 借走 `&mut system_context` **之前**调。
- **dispatch 在非借用期**：发生在 render（update+draw)完整返回后,此时 `ContextStack` 已 drop,闭包写 `State` 经 `try_write` 必成功 + Drop 唤醒 waker。事件分发与重绘解耦：重绘唤醒仍走 `use_state` 的 `poll_change`(与事件无关),故把 handler 从 poll_change 抽到中央分发器不破坏重绘。
- **dispatch 后无条件 continue**：复查 `should_exit`;纯副作用/退出型 handler 不写 State 不唤醒,否则 `select` 永久阻塞、exit 失效。退出经 `State<bool>` + `use_exit`(闭包 'static,捕获不到 `SystemContext`)。

**分层有序投递**：从层栈顶遇首个 `blocks_lower` 截断求活跃集;Phase 1 跑 `Global`(可 `Consumed` 截断、`Resize` 返 `Ignored` 不截断),Phase 2 层内按 `(z-order desc, priority desc, order asc)`——**z-order 第一键、不跨层比 priority**(否则下层 high 抢消费上层浮层)。

**不要做**：恢复 `Terminal` 的 `events()`/`wait()` 广播;在 update/draw 借用期 dispatch;把 z-order 排在 priority 之后。

**相关文件**：`crates/ratatui-kit/src/input/mod.rs`、`crates/ratatui-kit/src/hooks/use_input.rs`、`crates/ratatui-kit/src/render/tree.rs`、`crates/ratatui-kit/src/terminal/mod.rs`

### ScrollView 事件语义 + 几何(scrollview-overhaul 后)

`ScrollView` 的内置滚动 handler 现在对**真正处理了的**滚动键/滚轮返回 `EventResult::Consumed`(其余 `Ignored`),由 `ScrollViewState::handle_event(&event) -> bool` 驱动。`state`(外部状态)与 `active`(默认 true)**正交**——传 state 不再关掉内置滚动。

**内嵌可选择子组件仍建议外部驱动**:把 `Table`/`Select` 塞进 ScrollView 时,让子组件 `active: false`,由父级 handler 驱动选择并对导航键返回 `Consumed`(父 handler 登记在前,截断分发,ScrollView 收不到这些键);`PageUp/PageDown/滚轮` 留给 ScrollView。见 `examples/components/table.rs`。要让选中联动滚动,用 `ScrollViewState::scroll_to_visible(y, height)`(state 层原语已具备;把「子 key/index → 缓冲区 y」映射接上是后续工作)。

**几何关键点(scrollbars.rs / mod.rs)**:
- **内区单一真源**:`UseScrollImpl` 持 `Block`,`pre_component_draw` 捕获 outer、`post` 用 `block.inner(outer)`,与 `draw()` 完全一致(对部分边框/padding/标题正确)。勿再硬编码 `+1/-1/-2` 内缩。
- **视口感知裁剪**:偏移与 `page_size` 按「扣掉已显示滚动条后的视口」算(inset 模式),否则有滚动条时最后一行/列滚不到(上游 0.6.7 修的坑)。
- **`over_border` 开关**(`Scrollbars::over_border`,默认 true):ring 模式滚动条画在 block 边框环上、不占视口(视口=inner);inset 模式画在 inner 内、占一行/列。ring 仅当 `over_border && inner 右/下各有边框`。
- **嵌套安全**:`ComponentDrawer.scroll_buffers` 是**栈**(push/pop),不是单槽;ScrollView 套 ScrollView 不再 `take().unwrap()` on None。

**相关文件**：`crates/ratatui-kit/src/components/scroll_view/{mod.rs,state.rs,scrollbars.rs}`、`crates/ratatui-kit/src/render/drawer.rs`、`examples/components/{scrollview,table}.rs`
