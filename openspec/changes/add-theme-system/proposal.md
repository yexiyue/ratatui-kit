## Why

当前 7 个组件在各自 `Props::default()` 里**各自硬编码**样式(Select/MultiSelect/Table 的黑底青高亮、SearchInput 的 yellow/green/red 状态色、DarkGray 边框、modal 的 `.dim()` 遮罩、confirm 的 `Cyan+BOLD`),颜色不成体系、无法统一;框架**没有主题概念**,用户既不能一处改全局观感,也无法运行时换肤,第三方组件更无从复用一套配色。借鉴 React context,引入**以共享 `Palette` 为唯一真源**的主题系统:全部组件默认样式从一处派生、可运行时响应式切换、第三方组件同一套机制接入且自动视觉协调。

本变更**不考虑向后兼容**——统一替换全部硬编码颜色,换取最简洁合理的架构。

## What Changes

- 新增**核心主题协议(always-on,不门控,零新依赖)**:
  - `Palette`:自底向上、恰好够用的语义色板 —— `bg / surface / overlay`、`fg / fg_dim`、`accent / on_accent`、`selection`、`border / border_active`、`success / warning / error / info`、`placeholder`。`#[non_exhaustive]`。
  - `ComponentTheme` trait + 每组件一个 `FooTheme`(`SelectTheme` / `TableTheme` / `ModalTheme` ...),各自实现 `from_palette(&Palette) -> Self`;**颜色**一律取自 `Palette`,**非颜色决定**(高亮符号、`DIM` 遮罩、选中 `BOLD`、黑字配强调底的配对)由各组件的 `from_palette` 承接,从而颜色统一而结构自主。
  - **解析链**(每组件一致):显式 `FooTheme` override context → `FooTheme::from_palette(&palette)` → `FooTheme::default()`。
  - `PaletteProvider` / `ThemeOverride` 组件 + `use_palette()` / `use_component_theme::<T>()` hooks,复用现有 `ContextProvider` / `use_context` / `ComponentUpdater::get_context`;手写组件读取后**先 clone、drop guard,再 `update_children`**(规避 `AlreadyBorrowed` panic 与 `Ref` vs `&mut updater` 借用冲突)。
- **BREAKING**:所有组件的 per-call 样式 props 由 `Style` 改为 `Option<Style>`(`None` = 用主题;`Some(_)` = `resolved.patch(props)` 覆盖),配 `impl Into<Option<Style>>` 保留 `element!` 裸写手感。消灭 `Style::default()` 被当"继承哨兵"的静默陷阱。
- **BREAKING**:移除 7 个组件 `Props::default()` 中的全部硬编码颜色/修饰符,默认观感改由 `Palette` 派生;组件 apply 路径由 **overwrite 重写为 `resolved.patch(props)`**。
- **BREAKING**:`TreeSelect` / `VirtualList` 等原本样式中性(选中态默认不可见)的组件,选中态改由主题提供 —— 主题化后默认可见。
- **运行时换肤**:主题源可置于 `Atom<Palette>` / `use_state`,写入经 Waker 唤醒整树重渲,实现响应式切换;`PaletteProvider` 每帧从 prop 重注入。
- **特性门控(非 always-on)**:v1 只新增 `serde`(保存/加载 `Palette` 偏好)。内置预设主题包、`ratatui-themes` / `ratatui-themekit` 适配器列为后续真实内容,届时再加核心 feature,不提前放空 feature 位。**v1 仅交付协议 + 一套统一默认主题**。
- 新增 `examples/theme.rs`:演示全局主题、组件级 `ThemeOverride`、运行时切换、`Option<Style>` per-call 覆盖。

## Capabilities

### New Capabilities

- `theme-system`:框架级主题能力。涵盖 `Palette` 语义色板(唯一真源)、`ComponentTheme` trait 与每组件 `FooTheme` 的 `from_palette` 派生、三级解析链(override context → palette 派生 → default)、`PaletteProvider` / `ThemeOverride` 与 `use_palette` / `use_component_theme` 两组 API、`Option<Style>` per-call 覆盖的 `patch` 合成语义、内置组件默认观感的统一来源、以及 `Atom<Palette>` 驱动的响应式换肤;含无 Provider 兜底、手写组件读取纪律与 render-harness 样式断言约定。

### Modified Capabilities

- `extension-api-surface`:承诺 semver 稳定的公共扩展面枚举 MUST 增列主题协议公共项(`Palette`、`ComponentTheme`、`use_palette` / `use_component_theme`、`PaletteProvider` / `ThemeOverride`),使第三方组件作者可依赖主题接入面。

## Impact

- **框架核心**:新增 `components/theme` 模块(`Palette`、`ComponentTheme`、各 `*Theme`、`from_palette`、解析 hook),并从根导出层继续暴露主题稳定面;`prelude` 增出主题项;复用 `context.rs` / `components/context_provider.rs` / `hooks/use_context.rs` / `render/updater.rs::get_context`,无新增运行时依赖。
- **内置组件(BREAKING 面)**:7 个硬编码组件(`select` / `multi_select` / `table` / `search_input` / `alert_modal` / `confirm_modal` / `shortcut_info_modal`)+ 中性组件(`tree_select` / `virtual_list`)迁移到解析链;每个样式 slot 的 apply 路径由 overwrite 重写为 `resolved.patch(props)`,per-call props 全改 `Option<Style>`。
- **特性**:`crates/ratatui-kit/Cargo.toml` 新增 `serde` feature;协议本体 always-on(被 `Text`/`Border`/`Modal` 等未门控组件消费,无法门控且零新依赖),仅 `serde` 引入依赖。预设/适配器不在 v1 放空 feature 位。
- **测试**:`render/harness.rs` 增加 per-cell `Style` 断言能力;新增单测覆盖「无 Provider 默认观感」「Palette 生效」「组件级 override」「`Option<Style>` per-call 覆盖」「`Style::reset()` 清空」「运行时切 `Palette` 触发重渲」。
- **生态就绪**:已核实 `ratatui-themes` v0.2.0 与 `ratatui-themekit` v0.6.1 均兼容 ratatui 0.30,后续适配器 feature 不会重演 textarea 的版本钉死问题。
- **非目标**:不做兼容层 / `Theme::legacy()`;不新建 `ratatui-kit-contrib` crate;v1 不含成套预设主题与第三方适配器(后续 feature);不引入亮/暗双模(单档配色)。
- **运行时不变量**:保持 `poll_change` 三路全 poll、`State`/`AtomState` 的 `Send + Sync`、context 查找三态语义;主题为纯值/context,不改这些约束。
