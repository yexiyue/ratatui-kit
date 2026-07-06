## Why

`ScrollView` 是从上游 `tui-scrollview` 移植来的,但组件层(block 支持、事件接线、视口/裁剪几何)是自研且写得较急。一次多智能体对抗式评审(29 agents,全 3:0 verified)确认了 4 个真 bug、一簇与全库不一致的 API、以及若干复用障碍;同时用户明确希望「滚动条盖在边框上」成为可切换的观感。这些问题彼此耦合(block 内区几何同时影响裁剪、page_size、滚动条位置),适合一次性系统性修好,而不是零敲碎打。

## What Changes

- **修正 block 内区几何(单一真源)**:`pre_component_draw` 硬编码 `x+1,y+1,width-1,height-2` 与 `draw()` 的 `block.inner()` 不一致——右边框被内容覆盖、且对带 padding/标题/非四面边框的 block 全错。改为二者共用同一 `block.inner()`。
- **新增 `over_border` 开关**:让每个实例自选滚动条画在 block 边框上(默认,观感更好)还是退到框内;两轴对称;盖边框时内容 blit 严格限制在 inner 内,无滚动条方向保留边框。
- **视口感知的偏移裁剪与 page_size**:裁剪与翻页按「扣掉已显示滚动条后的视口」计算(对齐上游 0.6.7 的修复),修复「有滚动条时最后一行/列滚不到」与翻页重叠错乱。
- **修复嵌套 ScrollView panic**:`ComponentDrawer.scroll_buffer` 单一共享槽在子绘制前后 save/restore,消除 `take().unwrap()` on None。
- **补齐上游能力**:移植 `is_at_bottom()`;为 `ScrollViewState` 增加 `size()` / `page_size()` 只读 getter。
- **新增 scroll-to-visible / ensure_visible**:让可选择子组件的选中项能联动滚动进视口(解决 Table 选中行滚出视口),并提供「子 key/index → 缓冲区 y」的映射入口。
- **修正键盘滚动的作用域**:auto 模式键盘滚动当前是层级全局的(hit_test 只管鼠标),与内嵌 Input / 多个 ScrollView 串扰;改为按聚焦/激活门控,并让处理过的滚动键返回 `Consumed`。
- **API 一致性(BREAKING)**:`scroll_view_state` → `state`;`disabled: bool` → `active: bool`(默认 true,且传外部 state 时不再静默关闭内置滚动——两模式改为正交);`ScrollBars` → `Scrollbars`(与 `Scrollbar`/`ScrollbarVisibility` 一致),`scroll_bars` 字段随之 → `scrollbars`;`ScrollViewState::handle_event` 返回 `EventResult`。
- **文档**:记录内容缓冲区的 u16 上限与「每帧全量绘制」代价,给出 ScrollView vs VirtualList 的选择指南。
- 修正 `Scrollbars` 允许覆盖 `Scrollbar` 值但渲染硬编码 VerticalRight/HorizontalBottom 的隐患(内部固定朝向,仅放开符号/样式覆盖)。

## Capabilities

### New Capabilities
- `scroll-view`: `ScrollView` 组件与 `ScrollViewState` 的行为契约——block 内区几何与 `over_border` 开关、视口感知的滚动/裁剪/翻页/`is_at_bottom`、scroll-to-visible、滚动条配置与朝向、嵌套安全、事件处理与作用域,以及公开 API 形状(`state`/`active`/`scrollbars` 命名)。

### Modified Capabilities
<!-- 无:现有 specs 均未涉及 ScrollView 行为契约,extension-api-surface 也未枚举其 props。 -->

## Impact

- **代码**:`crates/ratatui-kit/src/components/scroll_view/{mod.rs,state.rs,scrollbars.rs}`;`ComponentDrawer`(`render/drawer.rs`)的 `scroll_buffer` save/restore 约定;可能触及 `InstantiatedComponent` 的 pre/post draw 时序理解(不改行为)。
- **公开 API(BREAKING)**:`ScrollViewProps` 字段重命名(`scroll_view_state`→`state`、`disabled`→`active`、`scroll_bars`→`scrollbars`)、类型重命名(`ScrollBars`→`Scrollbars`)、`handle_event` 返回值变更。需同步 `prelude` 导出、`examples/components/{scrollview,wrapped_text,table}.rs`、`components/shortcut_info_modal.rs`,以及 docs 站与 skill 的组件文档。
- **依赖**:无新增依赖(纯 ratatui/crossterm)。
- **测试**:新增 `scroll_view` 的渲染 harness 测试(over_border 两模式、有/无滚动条的裁剪与 page_size、嵌套 ScrollView 不 panic、is_at_bottom、scroll-to-visible)。
- **文档站/skill**:更新 ScrollView 组件页(en/zh)与 skill `components.md` 的 ScrollView 条目以反映新 API 与开关。
