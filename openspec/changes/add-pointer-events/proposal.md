# Proposal

## Why

框架的事件管线已能承载 `Event::Mouse`（`InputRuntime.dispatch` 甚至已实现 `hit_test` 矩形命中过滤，ScrollView 滚轮即在用），但**终端鼠标捕获从未开启**——`CrossTerminal::new()` 走 `ratatui::init()`，不含 `EnableMouseCapture`，鼠标事件实际到不了运行时。同时现有同层分发序的 tie-break 是注册序（update 自顶向下 = 父先于子），对命中类事件是**反冒泡**的：父组件消费点击会饿死子组件，遮罩点击关闭、卡片内按钮等父子叠加交互无法正确表达。作为 UI 框架，缺少点击 / 悬停 / 拖拽这些最基本的指针交互能力。

## What Changes

- **显式启用鼠标捕获**：新增 `RunOptions { mouse: bool }`（默认 `false`）与 `ElementExt::fullscreen_with(RunOptions)` / `render_loop_with(TerminalOptions, RunOptions)`；启用时执行 `EnableMouseCapture`，退出经现有 `RestoreGuard` 恢复。不启用则现有纯键盘应用行为完全不变。未启用而注册指针 hook 时，debug 构建下断言并给出指引信息。
- **指针事件冒泡**：鼠标命中链按**绘制序倒排**（`draw_seq desc`，由 `pre_component_draw` 回填区域时盖章）作为层内 tie-break——子先于父、后画者先于先画者；`Ignored` 向祖先 / 下层传播，`Consumed` 即截断。回填时同步盖**帧戳**，分发时帧号不匹配当前帧视作未命中（防「组件本帧未绘制、旧区域幽灵命中」）。键盘事件排序不变。
- **指针捕获路由**：hit_test handler 消费 `Down` 后，runtime 记住（handler token, button），该按钮后续 `Drag` / `Up` 绕过命中过滤与排序**独占直送**捕获者，`Up` 后清除——拖拽出区域事件不丢。
- **hover 全局 diff**：`Moved` 事件仅在 runtime 集中计算 hover 目标集合变化，enter / leave 之外不产生重绘副作用；所需 any-motion（`1003h`）在存在 hover 注册时惰性启用（只升不降）。
- **新 hooks（语义化指针 API）**：`use_click`（Down+Up 同目标合成 `Click { count, .. }`，默认 `Consumed`）、`use_hover() -> bool`、`use_drag`（Start / Move / End 三态，捕获内建）、`use_pointer`（完整 `PointerEvent` 流，`EventResult` 自控）。
- **新组件 `Button`**：`on_press: Handler<'a, ()>`（左键 `Down` 触发，终端惯例）、`variant`、`disabled`、内置 hover 高亮。无 feature 门控（沿用 input-layer 设计 D1「事件运行时无 feature 门控」；零新依赖）。
- **已知限制（不在本期范围）**：嵌套 ScrollView 滚轮穿透、命中区域与视觉裁剪求交、点击驱动的键盘 focus 环。

## Capabilities

### New Capabilities
- `pointer-events`: 指针（鼠标）交互能力——捕获启用、命中冒泡传播、点击 / 双击 / 悬停 / 拖拽语义、指针捕获路由，以及承载这些语义的 `Button` 组件与语义化 hooks 的对外行为契约。

### Modified Capabilities

（无。主 specs 中不存在 input-events 能力；`input-layer-event-system` 为 in-flight 变更的 delta，本变更不修改它，仅在其已实装的 `InputRuntime` 之上扩展。）

## Impact

- **代码**：`crates/ratatui-kit/src/` 下 `input/`（分发排序、帧戳、捕获路由、hover diff）、`terminal/`（捕获执行与恢复）、`render/tree.rs`（`RunOptions` 贯穿）、`context.rs`（`mouse_enabled` 标志）、`hooks/`（四个新 hook）、`components/`（`Button`）、`element/element_ext.rs`（新入口方法）。
- **API**：纯增量，无破坏性变更；`fullscreen()` / `render_loop()` 原签名与行为不变。
- **依赖**：零新增（crossterm 经 ratatui 已在依赖树）。
- **示例 / 测试**：新增 mouse 示例；单测覆盖冒泡序、重叠兄弟、帧戳、捕获路由、click 合成；沿用 `--all-features` CI 矩阵（默认 feature 集为空，无门控面）。
