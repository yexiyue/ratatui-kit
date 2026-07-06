## Context

`ScrollView` 移植自上游 `tui-scrollview`,但组件层(block 支持、事件接线、视口几何)是为 ratatui-kit 保留树运行时自研的。评审(29 agents 对抗验证)确认:内区几何在 `pre_component_draw`(硬编码 `+1/+1/-1/-2`)与 `draw()`(`block.inner()`)两处不一致;偏移裁剪与 `page_size` 用了原始区而非扣掉滚动条的视口(上游 0.6.7 已修此类);嵌套时共享的 `ComponentDrawer.scroll_buffer` 槽会 `take().unwrap()` on None 而 panic;公开 API 命名与 `Select/Table/VirtualList` 不一致且两种使用模式互斥。用户另要求「滚动条盖在 block 边框上」成为可切换观感。

约束:单线程渲染;`Block` 0.30 起非 `Send+Sync`,可直接持有 `Option<Block<'static>>`(见 macros-and-props 知识);pre/post_component_draw 时序为 `pre → 组件 draw → 子 draw → post`;本项目改动直提 main,可接受破坏性 API 变更(趁早改代价小)。

## Goals / Non-Goals

**Goals:**
- 内区几何单一真源:`pre_component_draw` 与 `draw()` 用同一 `block.inner()`,对任意 block(部分边框/padding/标题)一致。
- `over_border` 开关:滚动条盖边框(默认)/退框内两种观感,两轴对称,盖边框时不破坏无滚动条方向的边框。
- 视口感知的裁剪 + `page_size`,消除「最后一行/列滚不到」与翻页重叠错;据此正确移植 `is_at_bottom()`。
- 嵌套 ScrollView 安全(不 panic、不串帧)。
- `scroll_to_visible`/`ensure_visible` 让选中项联动滚动。
- 与兄弟组件一致的 API(`state`/`active`/`scrollbars`),两模式正交。
- 事件不再无声漏给兄弟 handler(`handle_event` 返回 `EventResult`)。

**Non-Goals:**
- 引入正式的「焦点(focus)」系统:auto 模式键盘仍作用于层内(缓解见 Decisions,彻底方案列 Open Questions)。
- 内容窗口化/虚拟化:超大内容仍推荐 `VirtualList`;本次只文档化 ScrollView 的全量绘制代价与 u16 上限。
- 改 `children` 声明式模型(评审确认其优于上游命令式 `render_widget`,保留)。

## Decisions

**D1 — 内区单一真源:hook 持有 `Block`,`pre_component_draw` 调用 `block.inner()`。**
`UseScrollImpl` 从 `has_block: bool` 改为持有 `block: Option<Block<'static>>`(或计算好的 inner Rect),`pre_component_draw` 用 `block.inner(drawer.area)` 得到与 `draw()` 完全相同的内区。*备选*:让 `draw()` 把算好的 inner 发布到 drawer 供 hook 读——被否,因为 pre 在 draw 之前触发,时序上拿不到;持有 Block 在 pre 阶段自算最简单且时序安全。

**D2 — `over_border` 开关的几何(挂 `Scrollbars`,默认 `true`)。**
以 `inner = block.inner()` 为基准:
- `over_border = true`:内容视口 = `inner` 全幅;滚动条轨道画在**边框环**上(纵向 = `inner` 右侧那列边框、横向 = 底部那行边框),两轴对称;内容 blit **严格裁剪在 `inner`** 内,故无滚动条的方向边框保留。滚动条**不占**内容视口 → 裁剪/`page_size` 用 `inner`。
- `over_border = false`:滚动条画在 `inner` 内,视口 = `inner` 扣掉已显示滚动条的一行/一列(上游模型)。
- 无 block(或该侧无边框):无边框可盖 → 退化为 inset 行为。
*备选*:把开关挂 `ScrollViewProps` 而非 `Scrollbars`——倾向挂 `Scrollbars`(滚动条外观归属),但最终位置在 specs 里锁定,保持与全库 props 习惯一致。

**D3 — 视口感知的裁剪与 `page_size`(统一 D2 的两模式 + 上游 0.6.7 修复)。**
`render_ref` 先解析 `show_horizontal/show_vertical`,再算 `viewport = over_border ? inner : inner - shownScrollbars`;偏移裁剪 `max_offset = content - viewport`,`state.page_size = viewport`(在 `render_scrollbars` 之后取,复用 `layout.visible_area`)。这样 D2 两模式与上游修复自然统一,`is_at_bottom`(依赖 `page_size` 语义)得以正确移植。

**D4 — 嵌套安全:save/restore 共享的 `scroll_buffer`。**
进入子绘制前保存外层 `drawer.scroll_buffer`(存到 `UseScrollImpl`),`post_component_draw` 取回本层 buffer 完成 blit 后**恢复外层**;`take().unwrap()` 改为带 guard(`if let Some(buf) = drawer.scroll_buffer.take()`)。*备选*:改成显式向下传递的局部 buffer(更彻底但要动 draw 管线与 `ComponentDrawer` 形状)——本次取最小侵入的 save/restore,并补嵌套渲染测试锁死。

**D5 — API 一致性(BREAKING)+ 两模式正交。**
`scroll_view_state`→`state`;`disabled: bool`→`active: bool`(默认 `true`);`ScrollBars`→`Scrollbars`、`scroll_bars`→`scrollbars`。去掉 `props.scroll_view_state.is_none()` 门控:`let state = props.state.unwrap_or(internal)`,内置滚动仅由 `active` 决定——传外部 state 不再关掉内置滚动,与 `Select/Table` 一致。`handle_event` 返回 `EventResult`,内置 handler 据此返回 `Consumed`/`Ignored`。

**D6 — 事件作用域缓解 + `Scrollbars` 朝向内固定。**
`handle_event` 命中滚动键/滚轮时返回 `Consumed`,不再无声漏给背景 handler(缓解与内嵌 Input/兄弟的串扰)。`Scrollbars` 只放开滚动条**符号/样式**覆盖,朝向由内部固定为 `VerticalRight`/`HorizontalBottom`,避免调用方设错朝向导致布局与渲染不一致。

## Risks / Trade-offs

- [over_border 盖边框时,某帧从「有滚动条」变「无滚动条」需正确重绘边框那一格] → blit 限制在 `inner`、边框由 `draw()` 的 `block.render` 每帧重画,滚动条仅覆盖其轨道格;补两模式 × 有/无滚动条的渲染测试。
- [D5 破坏 API,影响 examples/内置组件/docs/skill] → 直提 main、一次性改齐所有调用点(scrollview/wrapped_text/table example + shortcut_info_modal + prelude + docs en/zh + skill);tasks 里逐项列出。
- [auto 模式键盘仍层内广播,多 ScrollView/内嵌 Input 仍可能串扰] → 本次靠 `Consumed` 缓解 + 文档化「多滚动区/含 Input 时用手动模式 + 显式路由」;彻底 focus 方案列 Open Questions。
- [D4 若还有其它组件也用 `drawer.scroll_buffer`] → 目前仅 ScrollView 用;save/restore 后即便未来复用也安全。

## Migration Plan

1. 落 D1–D4(几何/开关/视口/嵌套),行为回归靠新渲染测试兜底。
2. 落 D5 破坏性重命名,同批改齐所有调用点与 `prelude`。
3. 补 `is_at_bottom` + getter + `scroll_to_visible`,并让 `table` example 用它实现选中联动滚动。
4. 更新 docs 站(ScrollView 页 en/zh)+ skill `components.md` 条目 + 录制/tape(如受影响)。
无独立回滚策略:直提 main,如需回退用 git revert 该批 commit。

## Open Questions

- 是否本次就引入最小「焦点」原语(如 `focused: bool` 或层内单活跃)来彻底解决 auto 模式键盘作用域?倾向下个 change,本次仅 `Consumed` 缓解。
- `scroll_to_visible` 的入口形态:按内容坐标 `y_range` 裁剪(通用、简单)还是按子 `key/index → 缓冲区 y` 映射(更贴 Table/Select 用例)?specs 里定接口,倾向先给按坐标的 `ensure_visible(y, height)`,组件侧再补 key→y 映射。
- `over_border` 默认值:定为 `true`(用户偏好)——如需与其它 TUI 惯例保持一致再议。
