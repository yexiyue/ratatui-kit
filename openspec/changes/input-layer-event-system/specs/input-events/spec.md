## ADDED Requirements

### Requirement: 单一事件源与中央分发

终端 SHALL 退化为单一 raw event 源（异步、泛型签名 `Terminal::<T>::next_event(&mut self) -> Option<T::Event>`，`None` 表示事件流结束），不再向多个订阅者广播。渲染循环取到一个 raw event 后,MUST 经唯一的中央分发器 `InputRuntime::dispatch` 投递给当前帧登记的事件 handler。框架 MUST NOT 把同一事件无差别 clone 给所有 handler。`Ctrl+C` 默认 MUST 在 `dispatch` 之前由 `TerminalImpl::received_ctrl_c` 判定并退出；应用显式关闭自动退出时，MUST 改由中央分发器投递。

#### Scenario: 渲染循环两分支
- **WHEN** 渲染循环 `select` 在「组件树变更」与「下一个 raw 事件」之间等待
- **THEN** 组件树变更分支仅回到重渲染;事件分支在自动退出开启时先判 `Ctrl+C`(命中则退出)、否则 `dispatch` 后无条件回到循环顶端复查退出条件;事件流结束分支退出循环

#### Scenario: 事件经中央分发而非广播
- **WHEN** 终端产生一个按键事件,且当前帧有多个组件登记了 handler
- **THEN** 该事件按层级/优先级/消费规则有序投递,而非同时投递给所有 handler

#### Scenario: 移除旧广播订阅
- **WHEN** 检索框架对外/对内 API
- **THEN** `Terminal::events()`/`Terminal::wait()`/`TerminalEvents`/`subscribers`、`use_events`/`use_local_events`、`UpdaterTerminal::events` 均不复存在

### Requirement: 事件注册表每帧重建

`InputRuntime` 的层栈与 handler 表 SHALL 在每帧 update 开始时（`begin_frame`)清空并重建:每个本帧参与的组件在其 update 期间重新登记层与 handler。因此被卸载的组件、关闭（`open=false`）的弹窗 MUST 在下一帧自动退出活跃集,无需显式注销,且 MUST NOT 发生注册表泄漏或身份串号。

#### Scenario: 关闭弹窗后背景自动恢复
- **WHEN** 一个 `blocks_lower` 弹窗从 open 切到 close
- **THEN** 下一帧该弹窗的层不再登记,背景组件的 handler 重新进入活跃集并响应事件

#### Scenario: 卸载组件的 handler 自动失效
- **WHEN** 某组件因协调（路由切换/条件渲染）在某帧不再被 update
- **THEN** 它的 handler 不再被登记,后续事件不会投递给它

### Requirement: InputLayer 独占栈与 blocks_lower 互斥

`InputRuntime` SHALL 维护一个按注册序（update 自顶向下 = z 序）排列的输入层栈。`dispatch` 计算活跃层集时 MUST 从栈顶向下,遇到第一个 `blocks_lower=true` 的层即截断:该层及其之上的层活跃,其下所有层（`Global` 除外)失活。

#### Scenario: 模态层屏蔽背景
- **WHEN** 背景列表在 root 层,弹窗注册了一个 `blocks_lower=true` 的层并位于栈顶
- **THEN** 该按键只投递给弹窗层（及 Global)的 handler,背景列表 handler 收不到

#### Scenario: 嵌套层只有栈顶活跃
- **WHEN** 弹窗层 L1 之上再注册输入子层 L2（`blocks_lower=true`,如输入框聚焦)
- **THEN** 仅 L2 的 handler 活跃,L1 的 handler 被截断

### Requirement: 事件消费 EventResult 截断传播

事件 handler SHALL 返回 `EventResult`（`Consumed` 或 `Ignored`,默认 `Ignored`)。`dispatch` 在有序投递过程中遇到 `Consumed` MUST 立即停止,不再投递给后续 handler;遇到 `Ignored` 则继续下一个。

#### Scenario: Consumed 阻止下游处理
- **WHEN** 输入框 handler 处理了 `Enter` 并返回 `Consumed`,而父列表也登记了 `Enter` handler
- **THEN** 父列表在本次分发中收不到该 `Enter`（替代旧的 `pending_exit` 延迟规避)

#### Scenario: Ignored 继续传播
- **WHEN** 一个 handler 对某事件返回 `Ignored`
- **THEN** 分发继续投递给候选集中下一个 handler

### Requirement: 分层有序投递（z-order 优先于 priority)

`dispatch` SHALL 分两个 phase。Phase 1 投递全部 `Global` handler,Phase 2 投递活跃层 handler。Phase 2 的排序键 MUST 为 `(层 z-order 降序, priority 降序, 注册序升序)`——**层 z-order 是第一排序键**:更靠栈顶（更高 z)的层的全部 handler MUST 整体先于更低层的 handler 被调用,即使下层 handler 的 `priority` 更高也 MUST NOT 抢先。同层内才按 `priority` 降序、再按注册序（update 自顶向下,父先于子)升序。`priority`/`order` MUST NOT 跨层比较。

#### Scenario: 上层非阻塞浮层不被下层高优先级抢消费
- **WHEN** 一个非阻塞上层浮层（`blocks_lower=false`)的 `Normal` handler 与下层背景的 `High` handler 同时活跃且都匹配某键
- **THEN** 上层浮层的 `Normal` handler 先被调用(z-order 优先于 priority);若它 `Consumed`,下层 `High` handler 不再被调用

#### Scenario: 同层内按优先级
- **WHEN** 同一层内一个 `High` 与一个 `Normal` handler 都匹配某键
- **THEN** `High` 先被调用;若它 `Consumed`,`Normal` 不再被调用

### Requirement: Global handler 独立 phase 与消费语义

`Global` handler SHALL 在层 handler **之前**（Phase 1)投递,按 `(priority 降序, 注册序升序)`,且 MUST NOT 受任何 `blocks_lower` 截断。`Global` handler 返回 `Consumed` MUST 终止整个 `dispatch`(截断后续层 handler);返回 `Ignored` 则继续投递层 handler。观察型全局监听（如 `Resize`)MUST 返回 `Ignored` 以不截断其它订阅者。

#### Scenario: 全局快捷键先于层 handler 并可截断
- **WHEN** 一个 `Global` 帮助键 handler 返回 `Consumed`,同时栈顶模态层也登记了同键 handler
- **THEN** 全局 handler 先被调用并截断,模态层 handler 不再收到该键

#### Scenario: 观察型全局监听不截断
- **WHEN** `Resize` 经 `Global` handler 投递并返回 `Ignored`,且存在多个尺寸订阅者
- **THEN** 所有订阅者都收到该 `Resize`,层 handler 的后续投递不受影响

### Requirement: 事件归属作用域 EventScope

每个 handler 登记时 SHALL 声明 `EventScope`:
- `Current` MUST 归属到 context 栈中最近的 `CurrentLayer`,无则归属 root 层；
- `Layer(handle)` MUST 归属到显式给定的层（用于 handler 注册在弹窗父组件、而弹窗自身经该 handle 开层的场景)；
- `Global` MUST 不受任何 `blocks_lower` 截断,任何帧都参与候选集。

#### Scenario: 父级 handler 经 Layer 显式归属，不失聪
- **WHEN** 弹窗的事件 handler 注册在 Modal 的父组件上,父组件 `use_input_layer(open, blocks_lower=true)` 取得 handle 并将 handler 声明为 `Layer(handle)`
- **THEN** 弹窗 handler 归属该 `blocks_lower` 层、随层成为栈顶而活跃,同时背景被截断——弹窗 handler 不失聪

#### Scenario: 子树 handler 经 Current 自动归属
- **WHEN** handler 注册在 Modal 的子树内,使用 `Current`
- **THEN** 它自动归属到 Modal 注入的 `CurrentLayer`,随 Modal 互斥

#### Scenario: Global 穿透所有层
- **WHEN** 一个 `Global` handler 与一个 `blocks_lower` 弹窗层同时存在
- **THEN** 弹窗打开时该 `Global` handler 仍收到事件

### Requirement: 非活跃层 handler 静默跳过

`use_input_layer(open=false, ..)` SHALL 仍返回一个有效的层句柄供同帧 `use_event_handler(EventScope::Layer(handle))` 绑定,但该层 MUST NOT 进入活跃层集。`dispatch` 遇到绑定到「未登记 / 非活跃」层的 handler MUST 静默跳过(不调用、不报错)。由此 `ConfirmModal`/`WarningModal` 在 `open=false` 时的语义明确:其层不登记 → 其 handler 不被调用,无需 `if !open { return }` 兜底。

#### Scenario: 关闭弹窗的 handler 不被调用
- **WHEN** 一个弹窗 `use_input_layer(open=false, ..)` 取得 handle、其 handler 以 `Layer(handle)` 绑定,此时有按键事件
- **THEN** 该 handler 因其层不在活跃集而被静默跳过,事件按其余候选正常分发

### Requirement: Modal 注册独占输入层

`Modal` 组件 SHALL 在 `open=true` 时注册一个输入层（默认 `blocks_lower=true`),并向其子树注入 `CurrentLayer`,使子树 handler 默认归属该层。`Modal` MUST 支持外部经 prop 注入层句柄（此时不重复注册层,仅注入 `CurrentLayer`),以支持「handler 在 Modal 父级」的场景。`open=false` 时 MUST NOT 注册层、MUST NOT 注入。

#### Scenario: 打开弹窗背景静默
- **WHEN** `Modal(open: true)` 渲染
- **THEN** 其子树 handler 活跃,背景组件键盘事件被屏蔽;关闭后背景恢复

#### Scenario: 非模态浮层不截断
- **WHEN** `Modal(open: true, blocks_lower: false)`（如非阻塞提示)
- **THEN** 它绘制在上层但不屏蔽下层 handler

#### Scenario: 外部注入层时父级 handler 与子树共享同一层
- **WHEN** 父组件 `let h = use_input_layer(open, true)` 取层、handler 用 `Layer(h)`,并将 `h` 经 `Modal(layer: Some(h))` 注入
- **THEN** Modal 不重复注册层、仅注入 `CurrentLayer(h)`;父级 handler 与 Modal 子树 handler 同属层 `h`,弹窗打开时一起活跃、背景被截断

#### Scenario: 漏传 layer 句柄导致父级 handler 失聪
- **WHEN** 父组件已 `use_input_layer` 并用 `Layer(h)` 绑定 handler,却忘记把 `h` 传给 Modal（`Modal(open: true)` 自开新层)
- **THEN** Modal 自开层位于 `h` 之上并截断 `h`,父级 handler 失聪——此为已知 footgun,API 与 example MUST 明确正确的三件套配对

### Requirement: use_input_layer 与 use_event_handler 钩子

框架 SHALL 提供 `use_input_layer(open, blocks_lower) -> InputLayer` 与 `use_event_handler(scope, priority, f)`（及 `use_event_handler_with_options`)两个 Sealed 钩子,取代 `use_events`/`use_local_events`。两者 MUST 在组件函数体内当帧登记到 `InputRuntime`,handler 闭包 MUST 每帧刷新（捕获最新 props/状态句柄)。`InputLayer` 句柄 MUST NOT 跨帧使用（每帧重新铸造)。

#### Scenario: 钩子调用顺序稳定
- **WHEN** 组件每帧调用 `use_input_layer`/`use_event_handler`
- **THEN** 它们占据稳定的 hook 顺序槽,符合「Hook 不放进条件/循环」规则;闭包每帧以最新捕获重建

#### Scenario: 登记守卫即弃
- **WHEN** 钩子在函数体内经 `SystemContext` 登记层/handler
- **THEN** 取得的可变借用守卫在登记后立即释放,随后子组件访问 `SystemContext`（如 `use_exit`)不会触发借用冲突 panic

### Requirement: 鼠标命中过滤选项

`use_event_handler_with_options` SHALL 提供 `hit_test` 选项复刻旧 `use_local_events`:当 `hit_test=true` 且事件为鼠标事件时,仅当指针落在 handler 所属组件区域内才调用闭包；键盘等非鼠标事件 MUST NOT 受 `hit_test` 影响。区域取自上一帧绘制,与历史行为一致。

#### Scenario: 区域外鼠标事件跳过
- **WHEN** `hit_test=true` 的 handler 收到一个落在其组件区域外的鼠标事件
- **THEN** 该 handler 不被调用,分发继续下一个候选

### Requirement: 退出与全局事件经安全通道

事件 handler 闭包为 `'static`,MUST NOT 直接访问 `SystemContext`/调用 `exit()`;退出意图 MUST 经写入响应式状态（`State<bool>`),由组件内 `use_exit` 落地。`dispatch` MUST 发生在 update/draw 之外的非借用期,闭包内写 `State` 安全。`dispatch` 之后渲染循环 MUST 复查退出条件（避免纯副作用 handler 不触发重绘导致退出失效)。`Ctrl+C` 在自动退出开启时 MUST 在 `dispatch` 之前判定,任何层的 `Consumed` 都不能吞掉它；自动退出关闭时 MUST 交由中央分发。`Resize` MUST 以 `Global` 投递且不被消费,使所有尺寸订阅者都能收到。

#### Scenario: 退出经状态生效
- **WHEN** 某 handler 写入退出状态 `State<bool>`
- **THEN** 该写入唤醒重绘,下一轮组件内 `use_exit` 读到并触发退出,渲染循环复查后结束

#### Scenario: Ctrl+C 默认不被消费吞掉
- **WHEN** 自动退出保持默认开启,且栈顶模态层的 handler 对所有键返回 `Consumed`,用户按下 `Ctrl+C`
- **THEN** 渲染循环在 dispatch 之前判定 `Ctrl+C` 并退出

#### Scenario: 应用接管 Ctrl+C
- **WHEN** 应用调用 `SystemContext::set_auto_quit_on_ctrl_c(false)`,并注册了 Ctrl+C handler
- **THEN** Ctrl+C 进入中央分发器,Global handler 可处理并返回 `Consumed` 阻止层内 handler 继续接收

#### Scenario: 弹窗打开时窗口仍可缩放
- **WHEN** 一个 `blocks_lower` 弹窗打开,终端窗口尺寸改变
- **THEN** `Resize` 经 `Global` 投递,尺寸订阅者更新,不被弹窗层截断
