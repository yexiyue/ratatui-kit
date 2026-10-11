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
  if !skip_render { render(); }
  if mouse && motion_requested { enable_any_motion(); }
  if should_exit break;
  select(component.wait(), terminal.next_event()).await;
  if event { auto_quit && ctrl_c ? break : input.dispatch(event); continue; }
}
```

`render()` 先自顶向下 `update`（跑组件函数体、跑 hooks、协调子树），再 `terminal.draw` 自顶向下 `draw`。然后 `select` 在「组件树有变化」与「终端有事件」之间阻塞。普通事件后重绘；无人消费且 hover 集合不变的 `Moved` 可跳过重绘，继续等待事件或状态唤醒。

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

### ScrollView 统一布局解析

ScrollView 在 draw 中解析一次 ResolvedLayout,子布局、视口、滚动偏移、轨道和最终 blit 共用该快照。枚举四种滚动条显隐候选选择自洽结果,不能在 post draw 重新计算。主轴 Percentage/Ratio/Fill 参照 block.inner 的原尺寸,交叉轴参照候选视口；尺寸计算饱和到 u16,负 gap 饱和为 0。偏移在子节点绘制前裁剪,确保命中与实际显示使用同一偏移。极小弹性布局若无显隐固定点,保守预留允许的轨道并裁掉空轨道。

### Context 查找区分「已借用」与「未找到」三态

`ContextStack::get_context(_mut)` 返回三态 `ContextLookup`（`Found` / `AlreadyBorrowed` / `NotFound`），而非 `Option`。断言型 `use_context(_mut)` 据此分别给「已被借用」（持守卫重入）与「未找到」（Provider 未注入）两种精确 panic 诊断；`try_use_context(_mut)` 与 `ComponentUpdater::get_context` 则把非 `Found` 一律降级为 `None`，**保持 try_/Option 接口永不 panic**。

`Context` 构造入口按所有权语义命名：`Context::owned(value)`、`Context::from_ref(&value)`、`Context::from_mut(&mut value)`。不要再写旧拼写 `form_ref` / `form_mut`。

**正确做法**：改 context 查找逻辑时保留三态——断言型给诊断、try_/Option 型安全降级。这与响应式状态的 `try_*` 约定一致（见 `hooks-and-state.md`）：`try_` 系列绝不 panic，只有断言型（`use_*` / `read` / `write`）才 panic。

**不要做**：把「已借用」的 panic 放进 `get_context` 这种被 `try_use_context` 复用的共享方法——会让 try_ 变体跟着 panic，破坏其非 panic 契约。

**相关文件**：`crates/ratatui-kit/src/context.rs`、`crates/ratatui-kit/src/hooks/use_context.rs`、`crates/ratatui-kit/src/render/updater.rs`

## 输入事件分发

### 中央 InputRuntime 分发取代广播订阅

事件系统从「广播订阅」(每个 `use_events` 各自订阅、所有 handler 平等收到同一事件)重写为「单 raw 源 + 中央 `InputRuntime` 分发」(`input/mod.rs`)。`Terminal` 退化为纯 raw source（`next_event`,删 `events()`/`wait()`/订阅者)；`render_loop`（`tree.rs`)取一个事件 → Ctrl+C 且 `SystemContext::auto_quit_on_ctrl_c()` 为真时直接退出 → 否则 `system_context.input.dispatch(event)`。应用层需自定义 Ctrl+C 时，每帧通过 `set_auto_quit_on_ctrl_c(false)` 关闭默认退出，并用 Global handler 优先处理该事件。

**正确做法**：理解三个不变量——
- **每帧重建注册表**：`update_once` 开头 `input.begin_frame()` 清空层/handler 并铸造 root 层,组件 update 期间经 `use_input_layer`/`use_event_handler` 重新登记。捕获 token、hover 集合与移动模式请求跨帧保留；层/handler 不跨帧复用，关闭的弹窗/卸载的组件下一帧自动退出。`begin_frame` 必须在 `ContextStack::root` 借走 `&mut system_context` **之前**调。
- **dispatch 在非借用期**：发生在 render（update+draw)完整返回后,此时 `ContextStack` 已 drop,闭包写 `State` 经 `try_write` 必成功 + Drop 唤醒 waker。事件分发与重绘解耦：重绘唤醒仍走 `use_state` 的 `poll_change`(与事件无关),故把 handler 从 poll_change 抽到中央分发器不破坏重绘。
- **dispatch 后 continue 并决定是否重绘**：复查 `should_exit`;未消费且 hover 不变的 Moved 才跳过重绘。退出经 `State<bool>` + `use_exit`(闭包 'static,捕获不到 `SystemContext`)。

**分层有序投递**：从层栈顶遇首个 `blocks_lower` 截断求活跃集；先更新 hover 集合，已有捕获的 Drag/Up 独占直送。其余事件 Phase 1 跑 `Global`(可 `Consumed` 截断、`Resize` 返 `Ignored` 不截断),Phase 2 层内按 `(z-order desc, priority desc, order asc)`；鼠标在 priority 后增加 `draw_seq desc` 再 `order asc`，键盘排序不变——**z-order 第一键、不跨层比 priority**(否则下层 high 抢消费上层浮层)。

### 鼠标模式在首帧登记后升级

crossterm 0.29 的 `EnableMouseCapture` 本身包含 `1003h`，不能用于按需开启 any-motion 的初始化。框架初始 Command 仅请求按压/拖动及坐标编码模式，再由 hover/track_move 请求启用 `1003h`。

**正确做法**：
- update/draw 后、首次等待输入前检查移动模式请求，否则首帧 hover 需要额外事件才能开始。
- 初始捕获与退出恢复配对；Windows native fallback 委托 crossterm，ANSI 路径使用受控序列。
- 序列测试验证初始化不包含 `1003h`，循环测试验证首次事件前完成升级。

**相关文件**：`crates/ratatui-kit/src/terminal/cross_terminal.rs`、`crates/ratatui-kit/src/render/tree.rs`

**不要做**：恢复 `Terminal` 的 `events()`/`wait()` 广播;在 update/draw 借用期 dispatch;把 z-order 排在 priority 之后。

**相关文件**：`crates/ratatui-kit/src/input/mod.rs`、`crates/ratatui-kit/src/hooks/use_input.rs`、`crates/ratatui-kit/src/render/tree.rs`、`crates/ratatui-kit/src/terminal/mod.rs`

### ScrollView 事件语义 + 几何(scrollview-overhaul 后)

`ScrollView` 的内置滚动 handler 现在对**真正处理了的**滚动键/滚轮返回 `EventResult::Consumed`(其余 `Ignored`),由 `ScrollViewState::handle_event(&event) -> bool` 驱动。`state`(外部状态)与 `active`(默认 true)**正交**——传 state 不再关掉内置滚动。

**内嵌可选择子组件仍建议外部驱动**:把 `Table`/`Select` 塞进 ScrollView 时,让子组件 `active: false`,由父级 handler 驱动选择并对导航键返回 `Consumed`(父 handler 登记在前,截断分发,ScrollView 收不到这些键);`PageUp/PageDown/滚轮` 留给 ScrollView。见 `examples/components/table.rs`。要让选中联动滚动,用 `ScrollViewState::scroll_to_visible(y, height)`(state 层原语已具备;把「子 key/index → 缓冲区 y」映射接上是后续工作)。

**几何关键点(scrollbars.rs / mod.rs)**:
- **内区单一真源**:`UseScrollImpl` 持 `Block`,draw 时解析 `block.inner(outer)` 并保存布局快照,post 直接使用该结果(对部分边框/padding/标题正确)。勿再硬编码 `+1/-1/-2` 内缩。
- **视口感知裁剪**:偏移与 `page_size` 按「扣掉已显示滚动条后的视口」算(inset 模式),否则有滚动条时最后一行/列滚不到(上游 0.6.7 修的坑)。
- **`over_border` 开关**(`Scrollbars::over_border`,默认 true):ring 模式滚动条画在 block 边框环上、不占视口(视口=inner);inset 模式画在 inner 内、占一行/列。ring 仅当 `over_border` 且 Block 确实含 RIGHT 和 BOTTOM 边框,轨道使用 outer 右/下边缘；padding 和底部标题不能冒充边框。Block 无边框 getter,仅 props 改变时用四种候选边框组合的 Eq 探测并缓存。
- **嵌套安全**:`ComponentDrawer.scroll_buffers` 是**栈**(push/pop),不是单槽;ScrollView 套 ScrollView 不再 `take().unwrap()` on None。

**相关文件**：`crates/ratatui-kit/src/components/scroll_view/{mod.rs,state.rs,scrollbars.rs}`、`crates/ratatui-kit/src/render/drawer.rs`、`examples/components/{scrollview,table}.rs`


### 绘制作用域与布局临时集合

父组件调用子节点 draw 后要恢复 calc_children_areas 完成时的 drawer.area,再运行父级 post_component_draw,避免最后一个子节点的区域泄漏。ScrollView 仍保留自己的内容缓冲 push/pop 生命周期。

普通布局和 ScrollView 通过内部 constraints iterator 直接遍历子约束,不再先收集临时 Vec 再遍历；叶子组件直接返回空区域集合。公开 get_constraints 仍保留显式收集入口。

### Context 作用域必须支持 panic 展开

with_context 内部把短借用 Context 暂挂到长生命周期栈,必须用 RAII 在正常退出和 panic 展开时恢复原栈深度,不能依赖闭包正常返回后的 pop。嵌套作用域各自恢复到进入前的深度。

### ScrollView 坐标映射与静止指针同步

Drawer 的内容缓冲栈同时组合屏幕变换和可见视口裁剪。Stamp 保存绘制区域与屏幕命中区域,PointerEvent.position 与 area 共用绘制坐标,screen_position 和原始 crossterm Event 保留终端坐标；use_drag 使用终端坐标计算连续拖拽位移。组件无需自行补滚动偏移。

InputRuntime 记住最后鼠标位置,绘制结束同步变化的 Enter/Leave 与 use_hover；内置列表和 use_hover_row 还重新核对内容映射,仅状态变化请求额外绘制。渲染循环先收敛这些更新再等待输入,避免静止指针悬停残留和每帧重绘循环。
