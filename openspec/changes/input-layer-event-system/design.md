## Context

当前事件系统是**广播模型**:`Terminal::wait`（`terminal/mod.rs`）把每个 crossterm 事件 clone 后推给所有 `use_events` 订阅者,各 handler 在自己的 `UseEventsImpl::poll_change` 里独立 drain 队列并执行闭包。没有焦点/层级/消费原语,输入无法互斥。

经逐行核验,确立两条**实现地基**:

- **地基 A（解耦)**:`UseStateImpl::poll_change`（`use_state.rs:60`）注册的是 `State` 的 `SingleWaker`,经 `InstantiatedComponent::poll_change`（`instantiated_component.rs:166-177`)的 hooks 路驱动「state → 重绘」,**与事件无关**。把事件 handler 从 `poll_change` 抽到中央分发器后,重绘唤醒完全不受影响。
- **地基 B（每帧重建)**:协调每帧用 `used_components` 重建 `Components`（`updater.rs:90-110`)。事件注册表同构地**每帧重建**——无跨帧持久状态,即无 push/pop 配对、无 `on_drop` 清理、无 `open:bool` 常驻 Modal 栈泄漏、无协调 key 复用串号。

**关键结构事实**:TRNovel 26 处 `use_events` 中约 15 处的 handler 注册在 **Modal 的父/兄弟组件**身上（`ConfirmModal`/`WarningModal`/`BrowserPromptModal` 在自身函数体 `use_events` 后 `return Modal{}`），不在 Modal 子树内。任何「Modal 向子树注入归属」的单一方案都够不到它们。

**约束（不可破坏）**:`poll_change` 三路全 poll（防丢唤醒）；`State`/`AtomState` 的 `Send + Sync`（`SyncStorage`）；context 查找三态（`use_context_mut` 是 **panic 版**，`updater.get_context_mut` 是降级 `None` 版）；`#[component]` 透明布局；运行时单线程渲染（事件 handler 闭包免 `Send + Sync`）。本设计**不考虑向后兼容**。

## Goals / Non-Goals

**Goals:**

- 框架级输入互斥:打开 Modal/进入输入态后,背景组件天然收不到键盘事件,无需应用层手算门控。
- 事件消费语义（`Consumed`/`Ignored`）+ 优先级,从根上消灭「同一 Enter 被多 handler 抢」(替代 `pending_exit`)。
- 单一可消费事件入口替代广播,事件分发与重绘唤醒解耦。
- TRNovel 删除 `is_inputting` 全局 bool、`pending_exit`、各 `!modal_open` 手算门控。

**Non-Goals:**

- 不引入完整 Tab 焦点环/焦点遍历（同层多焦点仍由组件 `index`/`is_editing` 仲裁）。
- 不改 `poll_change` 三路全 poll 语义、不改 `State`/`Atom` 响应式模型。
- 不解决「同屏分屏双独立活跃域」（当前 LIFO 单活跃链已覆盖嵌套场景）。
- 性能优化非目标:仍每帧全树重绘（无脏标记）;本设计只解决正确性（互斥）。

## Decisions

### D1. 中央 EventDispatcher 替代广播（而非软门控）

新增 `InputRuntime`（`input/mod.rs`,无 feature 门控）挂在 `SystemContext`。渲染循环取到 raw event 后调 `input.dispatch(event)` 同步分发。**否决软门控**（在各 `poll_change` 里自我抑制）:软门控不能实现 `Consumed`/跨层截断,且仍让每个 handler 被唤醒。中央分发是实现 `blocks_lower`/`Consumed` 的唯一位置。

### D2. 注册表每帧重建（地基 B）

`update_once` 开头 `begin_frame()` 清空 `layers`/`handlers` 并铸造 root 层；update 期间各组件 append；分发用本帧快照。**否决持久注册表 + push/pop + `on_drop` 清理**:那是对抗验证中所有生命周期坑（栈泄漏/串号/复用）的来源。每帧重建闭包的开销与现状 `use_events` 每帧 `Box::new(f)` 完全相同。

### D3. 统一登记路径 = 函数体内经 `SystemContext` 当帧登记（红队必改 2）

核验 `component.rs:184-189`:`#[component]` 生成的 `update` 体中,**函数体（含所有 hook 调用)执行在 `update_children`（最后一步)之前**。因此 layer 与 handler 都在**函数体内**经 `hooks.use_context_mut::<SystemContext>().input` 当帧登记,天然满足「严格自顶向下、父先于子、layer 先于子 handler、当帧生效」。**否决 pre/post_component_update 钩子方案**:`post_component_update` 在子树 `update_children` 之后才跑,会导致「子 handler 先于父 layer 登记」的 `Current` 归属错配。唯一例外:鼠标 `hit_test` 的 area 回填留 `pre_component_draw` 钩子（draw 期无 context,用 `Rc<Cell<Rect>>` 共享句柄）。

### D3a. context-aware hooks 在手写 Component 中的前提（审核补漏)

`use_event_handler`/`use_input_layer` 内部经 `hooks.use_context_mut::<SystemContext>()` 登记,而 `Hooks::use_context_mut` 在 `context == None` 时**直接 panic**（`use_context.rs:40` 的 `expect("context not available")`)。关键事实:`AnyComponent::update`（`component/mod.rs:136`)把 `Hooks::new(...)` 构造的 `context == None` 的 hooks **原样转发**给组件;**只有** `#[component]` 宏（`component.rs:184-189`）在 implementation 执行前做了 `hooks.with_context_stack(updater.component_context_stack())`。因此:

- **函数组件**（`#[component]`):开箱即用——宏已升级 hooks。
- **手写 Component**（`ScrollView`、未来需要 context-aware hook 的)：MUST 在 `update` 体内先 `let mut hooks = hooks.with_context_stack(updater.component_context_stack());` 再调 `use_event_handler`,且把 hooks 操作与随后的 `&mut updater` 操作（`set_layout_style`/`update_children`)**时序分离**（ScrollView 现有结构天然满足:先全部 hooks 调用,最后才 `update_children`)。
- **Modal**:不经 hooks,直接 `updater.get_context_mut::<SystemContext>()`（降级 None 版),因为它要在 update 体内拿 layer id 注入子树。

**否决「框架层统一注入」**（Codex 推荐的「在 `InstantiatedComponent::update` 给所有组件传 context-aware Hooks」）:`Component::update(props, hooks, updater)` 签名**同时**持 `hooks` 与 `&mut updater`;若让 hooks 持 `&ContextStack`,与 `ComponentUpdater` 内 `&mut ContextStack` 借用冲突。函数组件能 context-aware,恰恰是因为宏把 implementation（只用 hooks)与 `update_children`（用 updater)在时序上分开;手写 Component 的 `update` 签名同时拿两者,无法在 trait 层统一注入（除非改 `Component::update` 签名移除 updater 的 context_stack 可变借用,波及所有组件、代价过大)。故采用「手写组件显式 `with_context_stack`」,与宏同款,**零框架核心改动**;`#[component]` 宏不变（函数组件靠宏、手写组件靠自身,两条路径不重叠,无重复包装)。

### D4. 三种归属作用域,`Layer(h)` 破解「弹窗失聪」（红队必改 4）

`EventScope { Current, Layer(InputLayer), Global }`:

- **Current**:读 context 栈最近 `CurrentLayer`,无则 root 层。用于背景列表、Modal **子树内** handler。
- **Layer(h)**:显式绑定到某层。用于 handler 在 **Modal 父级**的弹窗——父组件 `use_input_layer(open, blocks_lower=true)` 拿到 `h`,handler 用 `Layer(h)` 显式归属该层,并把 `h` 经 prop 传给一个**不自开层**的 Modal 注入子树。
- **Global**:不受 `blocks_lower` 截断（Resize、帮助键）。

这解决核心矛盾:若弹窗父级 handler 用 `Current`（=root 层）,弹窗自身 `blocks_lower` 层成栈顶会把 root 层截断 → 弹窗失聪。`Layer(h)` 让 handler 归属它显式开的层,与「背景静默」并存。

**否决「Modal 既自开层又让父 `Layer(h)` 指向它」**:父登记 handler 时子 Modal 的层尚未创建。裁决:Modal 自开层 + 注入 `CurrentLayer` 为**主路径**（覆盖子树 handler）;父级 handler 场景由「父开层 + 传 `h` 给不自开层 Modal」覆盖。

**Footgun(必须文档化 + example 覆盖,审核补漏)**:`Layer(h)` 路径要求父级**三件套配对**——`let h = use_input_layer(open, true); use_event_handler(Layer(h), ..); Modal(layer: Some(h), ..)`。**若忘记把 `h` 传给 Modal**,Modal 走主路径自开一个新层（z 更高、`blocks_lower`)入栈在 `h` 之上 → 截断 `h` → 父级 handler 失聪。canonical `input_mutex` example MUST 覆盖此配对模式;知识库记录此 footgun。

**inactive layer 语义（审核补漏)**:`use_input_layer(open=false, ..)` 仍 mint 并返回一个 handle 供同帧 `use_event_handler(Layer(h))` 绑定,但该层**不进** `layers` 栈。`dispatch` 候选过滤 `layer.is_some_and(|l| active.contains(&l))` 使「绑定到未登记/inactive 层」的 handler **静默跳过**(不调用、不报错)。这定义了 `ConfirmModal`/`WarningModal` 在 `open=false` 时的明确语义:层不登记 → handler 不被调用,无需 `if !open { return }` 兜底。

### D5. 分层有序投递:z-order 优先于 priority + Global 独立 phase（红队 + 审核修正）

`dispatch` 先判**活跃层集**:从 `layers` 栈顶向下扫,遇首个 `blocks_lower=true` 截断（含该层,更低层失活）。然后分两个 phase:

- **Phase 1（Global)**:所有 `is_global` handler,按 `(priority desc, order asc)` 依次调用。返回 `Consumed` 则**整个 dispatch 终止**(Global 可截断层 handler);返回 `Ignored` 继续。用例:帮助键（`Consumed`)、`Resize`/observer（`Ignored`,不截断,使所有订阅者都收到)。
- **Phase 2（层内)**:活跃层 handler,排序键 **`(layer z-order desc, priority desc, order asc)`** —— **z-order 是第一键**:更靠栈顶（更高 z)的层的 handler 整体先于更低层,即使下层 `priority` 更高也不能抢先;同层内才按 `priority`,再按注册序 `order`（自顶向下,父先于子)。遇 `Consumed` 早停。

`Modal` 默认 `blocks_lower=true`（独占)。**否决「只按 `(priority desc, order asc)` 忽略 z-order」**（原草案缺陷,Codex 第 2 点)：那会让**下层** high-priority handler 抢先于**上层**浮层的 handler 消费事件——非阻塞上层浮层（`blocks_lower=false`)会被下层背景的 high-priority handler 抢消费。z-order 必须是第一排序键。`order` 仅作同层同优先级的稳定 tie-break。

### D5a. 退出型与全局 handler 的 Consumed 边界

Phase 1 的 Global handler **遵守 `Consumed`/`Ignored`**:帮助键这类「全局快捷键」应返回 `Consumed`（吃掉该键、先于任何层);`Resize` 这类「观察型」MUST 返回 `Ignored`（不截断,令多个 `use_terminal_size` 订阅者都收到)。这是「Global 是否允许截断普通 handler」的明确裁决:允许,由 handler 自身的返回值决定;observer 用 `Ignored` 即不截断。

### D6. dispatch 借用安全 + `mem::take`（红队必改 3、5）

`dispatch` 发生在 `render()` 完整返回后、**非借用期**（`ContextStack` 已 drop,无 RefCell 守卫存活）,闭包写 `State` 走 `try_write` 必成功、Drop 时 wake → 下轮 `select` 立即重渲染。`dispatch` 内用 `std::mem::take(&mut self.handlers)` 取出遍历,消除「持 `&mut self.handlers` 调闭包」的自借用脆弱性。**借用纪律（硬性)**:任何「函数体内拿 `SystemContext` RefMut 登记」必须**块内即弃守卫**再 `update_children`——`SystemContext` 是全树共享单个 RefCell（`tree.rs:45`),不 drop 则子组件 `use_exit`（panic 版 `use_context_mut`）撞 `AlreadyBorrowed` panic。

### D7. dispatch 后无条件 `continue` 复查 `should_exit`（红队必改 1）

`render_loop` 的 Event 分支 `dispatch` 后无条件 `continue` 回 loop 顶,由 loop 顶 `render()` 后判 `should_exit`。纯副作用/退出型 handler 不写 State 不 wake,若不 continue 会 `select` 永久阻塞、exit 失效。**退出型 handler 必须写 `State<bool>` 由组件内 `use_exit` 落地**——闭包是 `'static Box`,捕获不到 `&mut SystemContext`,`'static` 边界天然阻断闭包内直接 `exit()`。

### D8. `InputLayer`/`LayerId` 每帧 mint,禁止跨帧（红队必裁）

`LayerId` 每帧 `begin_frame` 后单调 mint,`InputLayer{id}` handle 仅**同帧**父传子有效。**否决 `use_state(||LayerId::next())` 跨帧固定**:每帧重建模型下持久 id 无意义。`EventScope::Layer(h)` 的 `h` 每帧由 `use_input_layer` 重取。文档强警告:存进 `use_state` 跨帧用 → 下帧 id 不在 `layers` → handler 静默失聪。

### D9. Terminal 瘦身为纯 raw source + render_loop 两分支

删 `TerminalEvents`/`subscribers`/`events()`/`wait()`/`received_ctrl_c` 字段。新增**异步、泛型**签名 `pub async fn next_event(&mut self) -> Option<T::Event>`（`Terminal<T>` 泛型,`None`=事件流结束;不做 ctrl_c 检测、不广播)。`UpdaterTerminal` trait 删 `events()`,仅留 `insert_before`。`TerminalImpl::received_ctrl_c(event)`（纯关联函数)保留。

`render_loop` 的 `select(root_component.wait(), terminal.next_event())` 两分支:
- **`Left(())`**（组件树/状态变更)→ `continue`,仅回 loop 顶重渲染。
- **`Right(Some(event))`** → 先 `CrossTerminal::received_ctrl_c(event.clone())` 命中即 `break`;否则 `system_context.input.dispatch(event)`,再**无条件 `continue`**（红队必改 1,复查 `should_exit`)。
- **`Right(None)`** → `break`（事件流结束)。

ctrl_c 经 `TerminalImpl::received_ctrl_c` 在 `dispatch` **之前**即时判定,任何层的 `Consumed` 都吞不掉它。`render()` 内原 `if terminal.received_ctrl_c()` 检查删除。

### D10. 特殊事件:Resize 必须 Global

`use_terminal_size` 的 Resize 监听改 `use_event_handler(Global, Normal, 返回 Ignored)`——Resize 不可被 `blocks_lower` 截断（否则弹窗打开时窗口尺寸不更新),且返回 `Ignored` 让多个订阅者都收到。

### D11. 同层仲裁保留,跨层互斥交给层

层只解决**跨层**;**同层**仲裁保留现状:TTS 6 项保留 `index` 守卫（priority 无法表达「选中第几项」),各列表保留 `is_editing` 首行 early-return,`read_content` 保留 `is_scroll`。`SelectColor` 的 input+list 用**嵌套层**（Modal 层 L1 + SearchInput 输入层 L2 `blocks_lower` 截断 L1）。`is_inputting` 全局 bool 与 `pending_exit` 可**删干净**:`dispatch` 单次遍历 + `blocks_lower` 截断根治广播竞争。

## Risks / Trade-offs

- **[借用 panic 链]** 登记 input 的 `SystemContext` 守卫未即弃 → 子组件 `use_exit` panic。→ 所有 push 用 `{ let mut sys = ...; sys.input.push(..); }` 块内即弃;Modal 用 `updater.get_context_mut`（降级 None)同样先 drop 再 `update_children`。无单测,靠 example + 逐路径手验。
- **[read_novel toggle 失聪]** `i`/`t` 关闭键当前在页面 handler,弹窗 `blocks_lower` 截断页面层后关不掉。→ 关闭键**下沉进各 Modal 内部 High Current handler**;`ShortcutInfoModal` 改签名加 `on_close`,7 处调用方接线（新增逻辑,非纯迁移）。
- **[SearchInput 鸡蛋问题]** 触发输入态的 `s` 必须**常开 Current handler** 接（非仅输入态注册),否则进不去。→ handler 始终注册,内部按本地 `editing` State 分流;canonical example 验证。
- **[z 序隐式契约]** 同父多 Modal 的 z 序 = element 书写顺序,无编译期保护（read_novel 三层最脆弱)。→ 文档化 + `input_mutex`/read_novel 回归 example。
- **[InputLayer 跨帧 footgun]** 见 D8。→ API 文档强警告。
- **[Resize 必须 Global]** 见 D10。误用 Current 会被弹窗截断。
- **[area 滞后一帧]** `hit_test` 用 `pre_component_draw` 回填的**上一帧** area（与旧 `use_local_events` 的 `component_area` 完全一致),模态刚弹出首帧鼠标命中可能用旧 area。→ 可接受,无回归。
- **[迁移面广]** 26 文件 30 处 + Modal/ShortcutInfoModal/SearchInput 三共享组件改签名,无单测。→ 框架侧 T1–T10 全绿后再动 TRNovel;先验最复杂的 select_color 双层 + read_novel 三层。

## Migration Plan

分两阶段,框架侧自包含先行、可独立验证:

1. **框架核心（T1–T10)**:`input` 模块 → `SystemContext` 挂载 → `Terminal` 瘦身 → `render_loop` 改造 → `use_input_layer`/`use_event_handler` → 删 `use_events`/改 `use_size` → `Modal` 改造 → `ScrollView` 迁移 → canonical `examples/input_mutex.rs` → 迁移其余 examples。每步 `cargo build --all-features` + example 手验。
2. **TRNovel 全量迁移（T11–T15)**:共享组件改签名 → 删 `is_inputting` 链路 → 列表/导航组件 → 简单页面 → 复杂场景（select_color 双层/read_novel 三层/TTS)。
3. **知识库（T16)**:新增事件分发主题,更新 hooks-and-state/runtime-architecture/macros-and-props。

**回滚**:框架侧是不兼容大改,无渐进回滚;以 git 分支隔离,框架侧 T1–T10 全绿（四件套 + examples）作为合入门槛,未达标不动 TRNovel。

## Open Questions

- **z 序无编译期保护**:同父多 Modal 依赖 element 书写顺序定 z 序,是否需要给 Modal 增加显式 `z_index` prop 作为兜底?当前裁决:先靠书写顺序 + 文档 + 回归 example,不引入 `z_index`,待实际需要再加。
- **`hit_test` area 首帧滞后**:模态浮层刚弹出首帧的鼠标命中用上一帧 area。当前裁决:接受（与旧实现同语义,键盘互斥不受影响）。
