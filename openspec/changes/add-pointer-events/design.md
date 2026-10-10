# Design

## Context

本设计建立在 `input-layer-event-system` 已实装的核心之上（该变更尚 in-progress，但 `InputRuntime` 已在 `crates/ratatui-kit/src/input/mod.rs` 生效）：

- **中央分发器**：`render_loop`（`render/tree.rs:78-107`）`select` 组件唤醒与终端事件，事件经 `SystemContext.input.dispatch(event)` 同步分发。循环结构保证**两个事件之间必有重绘**——每轮迭代先 `render` 再等待下一个事件。
- **每帧重建**（地基 B）：`begin_frame` 清空层 / handler 表，组件 update 期间重新登记;`dispatch` 处理后归还 handler 表（跳过重绘的路径下,本帧 handler 集继续服务后续事件,见 D6）。
- **命中过滤已存在**：`EventOptions { hit_test }` + handler 持 `Rc<Cell<Rect>>` 共享句柄，`pre_component_draw`（`instantiated_component.rs:140`）draw 期回填区域，`dispatch` 内做矩形命中。
- **关键事实**：draw 遍历天然父先于子、兄弟按子列表序（`instantiated_component.rs:128-165`：`pre_component_draw` 先于子组件递归）。
- **约束**：运行时单线程（handler 闭包免 `Send + Sync`，但 props 回调 `Handler<'a, T, V>` 仍要求 `Send + Sync`）；`#[component]` 函数体先于 `update_children` 执行（自顶向下注册序 = 父先于子）；键盘事件分发语义不得变化。

调研参照：herdr（`github.com/herdrdev/herdr`）的中央命中表（`ShellHitMap` 渲染期记录 + 手写优先级链查询）验证了「渲染期 Rect 快照 + 事件期命中」在终端环境的可行性，但其命令式集中表与本框架的分布式组件注册模型不匹配，仅取其经验（Down 即触发、拖拽手势状态机、hover 变化才重绘、1003h 惰性开启）。

## Goals / Non-Goals

**Goals:**

- 指针事件具备正确的前端式传播语义：内层 / 视觉在上者优先，`Ignored` 向祖先冒泡，`Consumed` 截断（与键盘现有 `EventResult` 语义同构）。
- 提供分层 API：`Button` 组件（零成本声明式）→ 语义化 hooks（`use_click` / `use_hover` / `use_drag` / `use_pointer`）→ 原始 `use_event_handler_with_options`（逃生门，已有）。
- 拖拽、悬停、双击等合成语义开箱即用，且拖拽期间事件不因指针离开区域而丢失。
- 终端模式变更完全显式且可恢复；未启用时零行为差异。

**Non-Goals:**

- 不引入完整两阶段（capture → bubble）事件模型、`preventDefault`、focus 环 / 点击聚焦（延续 input-layer 的 Non-Goal）。
- 不做嵌套 ScrollView 滚轮穿透、命中区域与视觉裁剪求交（已知限制，见 Risks）。
- 不做 SGR 像素坐标（`1016h`）、多点触控、终端鼠标事件向子进程透传（multiplexer 场景）。
- 不改变键盘事件的分发顺序与语义。

## Decisions

### D1. 显式启用：`RunOptions { mouse: bool }`（否决 auto-enable 与 feature 门控）

新增 `RunOptions`（默认 `mouse: false`），经 `ElementExt::fullscreen_with(RunOptions)` / `render_loop_with(TerminalOptions, RunOptions)` 贯穿 `render_loop` → `Tree::new` → `SystemContext`（挂 `mouse_enabled` 标志）；启用时在终端 setup 阶段执行 `EnableMouseCapture`，退出由 `RestoreGuard`（`tree.rs:18-24`）追加 `DisableMouseCapture` 后再 `ratatui::restore()`。

- **否决注册即自动启用（棘轮）**：终端模式变更（原生拖选 / 滚轮回溯被劫持）是可观察副作用，深层子组件挂载不应隐式改变应用入口行为；显式入口可预测、可审计。开发构建下未启用而注册指针 hook 时 `debug_assert!` 并给出启用方法（防哑坑），release 下静默不投递（与现状一致）。
- **否决 feature 门控**：分发排序 / 帧戳是对现有 `hit_test` 用户（ScrollView）的正确性修复，应无条件生效；门控制造双代码路径。仓库惯例中 feature 服务于带外部依赖的可选组件（`input = ["tui-input"]`），鼠标零新依赖；且 input-layer D1 已确立「事件运行时无 feature 门控」。feature 回答「代码是否存在」，运行时开关回答「终端模式是否开启」——后者才是正确层次。

### D2. 冒泡 = 绘制序倒排（否决两阶段模型与集中命中表）

鼠标事件在层内的 tie-break 从注册序改为**绘制序倒排**：`pre_component_draw` 回填时同步盖全局单调 `draw_seq`，分发排序键变为 `(层 z desc, priority desc, draw_seq desc, order asc)`（**仅对 `Event::Mouse` 生效**，键盘保持 `(z, priority, order)`）。

- 子组件 `draw_seq` 恒大于父（父的 hook 回填先于子递归）→ 子先收到、`Ignored` 才轮到祖先 = 冒泡；后画兄弟先于先画兄弟 → 顺带修正重叠场景（视觉在上者先）。
- **否决前端式两阶段（capture → target → bubble）**：前端需要 capture 是因为监听器可挂任意节点且需双向遍历；单条「内层优先链」+ `Consumed` 截断已等价覆盖本框架场景，「先于视觉序」的需求由 `EventPriority` / 层 z-order 承担（现成逃生门），`EventScope::Global` 对应 document 级监听。
- **否决 herdr 式集中命中表**：集中表要求渲染函数边画边登记（命令式耦合），与函数组件 / hooks 模型冲突；现有 per-handler `Rc<Cell<Rect>>` 分布式机制已覆盖，且天然获得层 / 优先级 / 模态截断的组合能力，无需手写优先级链。
- 前端概念映射：target = 链首；bubble = `Ignored` 续传；`stopPropagation` = `Consumed`（已有）；`setPointerCapture` = D4；`preventDefault` = 不做（无默认行为）。

### D3. 回填单元升级为 `Stamp { area, frame, draw_seq }`（帧戳防幽灵）

现有 `Rc<Cell<Rect>>` 升级为 `Rc<Cell<Stamp>>`，`pre_component_draw` 一次性盖 `(area, 当前帧号, 全局 draw_seq)`。分发时**帧号 ≠ 当前帧视作未命中**。

- 解决「组件本帧未绘制但 hook 存活、旧区域幽灵命中」（条件隐藏、被遮挡跳绘），顺带覆盖「挂载后从未绘制」的默认值问题。
- `begin_frame` 递增帧号；`draw_seq` 由 draw 遍历自然产生，无需额外遍历。帧号 / seq 计数器放 `InputRuntime`（draw 期经 drawer 可达的共享句柄）——具体挂载点实现期定，约束是：update 期注册、draw 期盖章、dispatch 期校验，三者共用同一 `Stamp` 单元。
- 已知残余：命中区域滞后一帧（见 Risks）。

### D4. 指针捕获路由：hook 稳定 token + 独占直送

hit_test handler 消费 `Event::Mouse(Down)` 时，`InputRuntime` 记录 `(capture_token, button)`；该按键的后续 `Drag` / `Up` **绕过命中过滤与排序，独占直送** token 对应的当前帧 handler，`Up` 后清除。多按键各自独立成对。

- **跨帧身份**：handler 表每帧重建（地基 B），捕获身份不能用 handler 下标——由 hook 持有一次铸造、跨帧稳定的 `PointerToken`（`Rc` 指针身份或一次性 id），`HandlerEntry` 携带，与 `Rc<Cell<Stamp>>` 同款共享句柄模式。
- **生命周期**：捕获期间组件卸载 / handler 未再注册 → token 找不到对应 handler，捕获视为自动失效（等同清除）；不悬挂、不跨帧泄漏。
- **否决 hook 层自实现**：hit_test 过滤在 runtime，出区后的 `Drag`/`Up` 到不了 hook，hook 层无法自救；捕获必须是 runtime 能力（对应前端 `setPointerCapture` 的隐式版：消费 Down 即隐式捕获，不给用户显式 API，降低心智负担）。

### D5. 合成分工：runtime 只做全局必需，语义糖在 hook 层

| 能力 | 归属 | 理由 |
|---|---|---|
| 冒泡排序 / 帧戳 | runtime | 全局序 |
| 捕获路由 | runtime | 需绕过命中过滤（D4） |
| hover 目标集合 diff | runtime | `Moved` 洪泛需集中一次计算 |
| Click / 双击计数 | hook 层 | 纯局部状态机（记 Down 时间 + 位置，`Up` 时比对），窗口 ~300ms 常量，`Instant` 计时 |
| Drag 三态（Start / Move / End） | hook 层 | 基于 D4 路由结果的语法糖 |

`use_click` / `Button.on_press` 命中处理后默认 `Consumed`（「点我就是点我」直觉）；需要放行的场景走 `use_pointer` 自行返回 `EventResult`。`Click { count, button, position, modifiers }`；跨目标 Down / Up 不合成。

### D6. hover 全局 diff + `1003h` 惰性棘轮 + Moved 重绘抑制

- **1003h（any-motion）惰性启用**：`EnableMouseCapture`（含 `1002h` 按键拖动上报）在 D1 显式启用时执行；仅当当帧存在 hover / Move 追踪注册时追加 `1003h`（只升不降，避免模式抖动）。不支持 1003h 的终端自然降级：hover 无事件，其余正常。
- **集中 diff**：`Moved` 到来时 runtime 依据上帧 `Stamp` 集合计算最内命中链，更新各 token 的 hover 布尔；`use_hover()` 读取自身 token 的当前值。
- **重绘抑制**：`dispatch` 返回「是否需要重绘」——`Moved` 无人消费且 hover 集合无变化时返回 `false`，`render_loop` 跳过本轮 `render` 直接回到 `select`。安全性依赖现有唤醒机制：handler 若写 State，`root_component.wait()` 的 waker 会立即拉起重绘，不依赖每事件无条件重绘。契约：observer 型 handler 不得产生副作用（与 Global-Ignored 语义一致）。

### D7. `Button` 组件：Down 触发 + 内置 hover + 无门控

`ButtonProps { label, on_press: Handler<'a, ()>, variant, disabled, width: Option<u16>, style: Option<Style> }`，回调模式照抄 `ConfirmModalProps.on_confirm`;`width`/`style` 与 ConfirmModal 的尺寸/样式覆盖惯例一致（`None` 自适应宽 = 标签 + 4）。`on_press` 在左键 `Down` 触发（终端惯例，即时反馈、不依赖捕获、不受 Up 丢失影响）；内部 `use_click` 原语可后续演进为 Up 语义而不破坏 props。内置 `use_hover` 高亮与 `disabled` 灰显（disabled 时不注册命中、不高亮）。变体样式对接 theme-system，具体 token 实现期定。无 feature 门控（D1）。

### D8. 事件类型：合成层不入侵 crossterm `Event`

`use_pointer` 对外暴露自有 `PointerEvent { kind, button, position, modifiers, area }`（`PointerKind { Down, Up, Click, Drag, Move, Enter, Leave, ScrollUp/Down/Left/Right }`），由 hook 包装 raw `Event::Mouse` 合成；runtime 内部与 `use_event_handler` 逃生门继续流通 raw crossterm `Event`，不引入框架级新事件枚举——避免双事件模型并存导致的排序 / 消费语义分叉。

## Risks / Trade-offs

- [命中区域滞后一帧] 动画 / 跟随指针移动的组件命中恒差一帧 → 循环结构保证事件间必有重绘，静态与状态驱动布局无感；文档化为固有特性。
- [每事件必重绘 × Drag / Moved 洪泛] 捕获开启后拖拽期间 `Drag` 高频 → D6 抑制覆盖 `Moved`；`Drag` 属有效交互必然重绘，ratatui diff 渲染下成本可控（herdr 同规模验证）。
- [observer handler 在 Moved 上依赖无条件重绘] → 契约禁止 observer 副作用（沿 Global-Ignored 语义），文档化；违反者表现为视觉滞后一拍而非错误。
- [`Handler<'a, T, V>` 的 `Send + Sync` 约束] `Button.on_press` 闭包需 `Send + Sync`（与全部现有回调 prop 一致）；hook 层闭包不受约束（单线程）。
- [双击窗口 300ms 固定常量] 极端场景（远程高延迟）双击判定偏严 → 常量集中定义，后续可提为配置而不改架构。
- [捕获期间组件卸载] → D4 自动失效路径，配单测。
- [滚动穿透缺失] 内层 ScrollView 滚到边缘后滚轮被消费、不传外层 → 已知限制，后续在 ScrollView 边缘返回 `Ignored` 即可启用，无需框架改动。
- [命中区域不与视觉裁剪求交] 溢出被裁剪的子组件仍可命中其完整 rect → 已知限制；后续可在回填时与 drawer clip 求交。

## Migration Plan

纯增量变更，无迁移；`fullscreen()` / `render_loop()` 签名与行为不变。回滚 = revert 单个变更。发布说明需显著标注 `RunOptions` 用法与「未启用时指针静默」的诊断行为。

## Open Questions

- `Button` 的 `variant` 集合与 theme token 对接细节（实现期与 theme-system 对齐，不影响架构）。
- `PointerToken` 采用 `Rc` 指针身份还是一次性 id（实现期按借用检查友好度定，无行为差异）。
