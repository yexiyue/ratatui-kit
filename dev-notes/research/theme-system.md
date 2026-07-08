# Theme System（主题系统分层与生态调研）

## 概览

本主题记录 ratatui-kit 未来主题系统的分层边界、核心 API 方向，以及 `ratatui-themes` / `ratatui-themekit` 两个外部生态 crate 的定位。新增 `Theme`、`ThemeProvider`、组件默认样式 slot、主题预设包或主题适配器前先读本文件。

## 结论：协议进核心包，预设与适配进 contrib

**推荐分层**：

```text
ratatui-kit
  Theme / Palette
  ThemeProvider / use_theme()
  TextTheme / BorderTheme / InputTheme / SelectTheme / TableTheme / ModalTheme ...
  Theme::legacy() / Theme::no_color()

ratatui-kit-contrib
  内置主题预设：catppuccin_mocha / nord / dracula / tokyo_night ...
  ratatui-themes 适配器
  ratatui-themekit 适配器
  ThemePicker / theme cycling 示例与应用层小组件
```

核心包必须拥有主题协议，因为内置组件要消费主题默认样式。若主题系统完全放进 `ratatui-kit-contrib`，核心组件要么无法读取主题，要么需要反向依赖 contrib，形成依赖倒挂。预设主题、第三方生态适配、主题选择器等则应放 contrib，避免把审美选择、额外依赖、serde 配置和主题集合维护成本塞进核心。

**正确做法**：
- 核心只定义 typed contract：`Theme`、`Palette`、各组件 style slot、Provider/Hook。
- 核心保留 `Theme::legacy()` 作为当前视觉兼容默认值，保留 `Theme::no_color()` 处理无颜色环境。
- contrib 提供好看的主题预设、theme cycling、第三方 crate 转换函数和演示组件。
- 第三方主题 crate 只作为 palette / slot mapping 来源，不成为核心抽象的主人。

**不要做**：
- 不要让核心包依赖 `ratatui-themes` 或 `ratatui-themekit` 来定义主题协议。
- 不要把 Dracula / Nord / Catppuccin 这类主题集合直接放核心。
- 不要做 `HashMap<String, Style>` 作为第一版主题协议；字符串 token 拼写错误运行时才暴露，和当前组件 API 的类型化方向不一致。

## 核心主题模型建议

第一版主题应围绕「组件 slot」而不是只围绕颜色表：

```rust
pub struct Theme {
    pub palette: Palette,
    pub text: TextTheme,
    pub border: BorderTheme,
    pub input: InputTheme,
    pub search_input: SearchInputTheme,
    pub select: SelectTheme,
    pub multi_select: MultiSelectTheme,
    pub table: TableTheme,
    pub modal: ModalTheme,
}

pub struct Palette {
    pub foreground: Color,
    pub background: Color,
    pub muted: Color,
    pub border: Color,
    pub accent: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,
}
```

`Palette` 解决「语义色」，组件 `*Theme` 解决「这个组件的哪一块用什么样式」。例如 `palette.accent` 不能直接回答 `SearchInput.active_border_style`、`Table.horizontal_line_style`、`ConfirmModal.selected_button_style` 该怎么设置，必须由组件 slot 承接。

## 样式合成规则

Ratatui 的 `Style` 是增量合成模型：`Style::default()` 表示不修改已有样式，`Style::reset()` 才表示清空之前样式。主题系统应利用这个语义：

```rust
let style = theme.select.highlight_style.patch(props.highlight_style);
```

**推荐语义**：
- 主题 slot 先铺底。
- 用户显式传入的 props style 后 patch，覆盖主题。
- props 中 `Style::default()` 表示「不覆盖主题，使用主题默认」。
- props 中 `Style::reset()` 表示「清掉主题影响，回到终端默认」。

这会改变部分组件 `Default` 的含义：当前 `SelectProps::default()` / `TableProps::default()` 等直接写死 cyan/yellow/dark gray。主题化后，这些硬编码颜色应迁移到 `Theme::legacy()`，props 默认值尽量变成 `Style::default()`。这样无 `ThemeProvider` 时视觉保持不变，有 `ThemeProvider` 时主题能真正接管默认样式。

## Provider 与 Hook

核心可新增：

```rust
element!(ThemeProvider(theme: Theme::legacy()) {
    App()
})
```

实现上复用现有 `ContextProvider` / `use_context` 机制。建议 `use_theme()` 返回 `Theme` 克隆值，而不是把 `Ref<Theme>` 长时间交给组件持有：

```rust
let theme = hooks.use_theme();
```

这样能减少 `RefCell` guard 生命周期带来的借用摩擦。`Theme` 第一版字段主要是 `Style` / `Color` 等轻量值，`Clone` 成本可控。

手写 `Component`（如 `Border`、`Table`、`Modal`）不能直接用 context-aware hook，应在 `update` 里经 `ComponentUpdater::get_context::<Theme>()` 读取并立即 clone/drop guard，再继续 `update_children`。这与现有手写组件的 context 借用纪律一致。

## 组件接入顺序

建议按风险从低到高推进：

1. 低层组件：`Text`、`Border`、`Input`、`Modal`。
2. 组合输入组件：`SearchInput`、`ConfirmModal`、`AlertModal`、`ShortcutInfoModal`。
3. 选择组件：`Select`、`MultiSelect`、`TreeSelect`、`VirtualList`。
4. 复杂表格：`Table` 的 header/footer/row/highlight/border/separator slot。
5. 示例与文档：新增 `examples/theme.rs`，展示全局主题、局部覆盖、`Style::reset()`。

**正确做法**：
- 改组件时先把原硬编码默认值搬进 `Theme::legacy()`，再让组件从 theme slot 解析。
- 每个组件保留现有显式 style props，用 patch 覆盖主题。
- 用 render harness 检查 buffer cell style，至少覆盖 theme 生效、props 覆盖、reset 清除三类行为。

**不要做**：
- 不要一次性删除所有 style props；主题是默认来源，不是替代显式覆盖。
- 不要把业务文案、loading 空态、应用特定色名塞进核心 `Theme`。

## `ratatui-themes` 是什么

[`ratatui-themes`](https://docs.rs/ratatui-themes) 是一个 ratatui 主题颜色集合 crate。它提供：

- `ThemeName`：Dracula、Nord、Catppuccin、Gruvbox、Tokyo Night、Solarized 等主题名。
- `Theme`：包装当前主题名并提供 palette 访问。
- `ThemePalette`：包含 `fg`、`bg`、`accent`、`muted`、`error`、`warning`、`success`、`info` 等语义色。
- theme cycling：`next()` / `prev()` / `all()`，适合应用内主题切换。
- serde 支持：适合保存用户主题偏好。
- 可选 widgets：例如主题选择器。

它的价值是「快速拿到一套流行主题 palette」。它的问题是「不知道 ratatui-kit 的组件 slot」。`palette.accent` 可以用来推导 `border_style` 或 `highlight_style`，但具体映射规则仍应由 ratatui-kit 的 `Theme` / contrib adapter 决定。

**适合放在 contrib 的形态**：

```rust
pub fn from_ratatui_themes(theme: ratatui_themes::Theme) -> ratatui_kit::Theme;
pub fn from_ratatui_theme_name(name: ratatui_themes::ThemeName) -> ratatui_kit::Theme;
```

注意：`ratatui-kit-contrib` 不能实现 `From<ratatui_themes::Theme> for ratatui_kit::Theme`，因为 `From`、外部主题类型、`ratatui_kit::Theme` 对 contrib 来说都不是本地定义，违反 Rust orphan rule。用普通转换函数，或定义 contrib 自有 extension trait。

## `ratatui-themekit` 是什么

[`ratatui-themekit`](https://docs.rs/ratatui-themekit) 是一个更靠近 widget 使用层的 semantic theme 工具箱。它提供：

- `Theme` trait：一组语义色 contract。
- `ThemeData` / `CustomTheme`：以纯数据或配置定义主题。
- 内置主题：Catppuccin、Dracula、Gruvbox、Nord、OneDark、Rose Pine、Solarized、Tailwind Dark、Tokyo Night、Terminal Native、NoColor 等。
- `ThemeExt`：给主题增加 builder 方法。
- builder/style bundles：`ThemedSpan`、`ThemedLine`、`ThemedBlock`、`ThemedStatusLine`、`TableStyles`、`ListStyles`、`InputStyles`、`TabStyles`、`ScrollbarStyles` 等。

它比 `ratatui-themes` 更「框架化」：不只是 palette，还提供面向 ratatui 原生 `Span` / `Line` / `Block` / widgets 的 builder 和 style bundle。它适合给 ratatui 应用直接减少样式样板，也适合作为 ratatui-kit 组件 slot 设计的参考。

它不适合作为 ratatui-kit 核心主题协议的直接依赖，原因是它的 builder 面向原生 ratatui primitives，而 ratatui-kit 需要的是内置组件 slot contract。核心若直接绑定它，组件主题命名和演进会被外部 builder API 牵制。

**适合放在 contrib 的形态**：

```rust
pub fn from_themekit<T: ratatui_themekit::Theme>(theme: &T) -> ratatui_kit::Theme;
```

或提供少量预设函数，把 `ratatui-themekit` 的内置主题映射到 ratatui-kit slot。

## 与未来 `ratatui-theme` 的关系

[`ratatui-theme`](https://docs.rs/ratatui-theme) 当前是 namespace reservation，文档说明它是 Ratatui 未来 styling/theme 方向的占位 crate，尚无公开 API。ratatui-kit 不应等待它落地，也不应假设其未来模型。但核心主题协议应保持简单、类型化、以 `ratatui::style::Style` 为底层值，这样未来若上游出现稳定模型，可以在 contrib 或 optional feature 中做适配。

## 待决问题

- `Theme::legacy()` 是否就是 `Default`？推荐是：`Default` 返回 legacy，避免无 Provider 视觉变化。
- 是否需要 `Theme::auto()` 读取 `NO_COLOR`？推荐先提供 `Theme::no_color()`，环境读取放应用层或 contrib helper，避免核心在构造默认值时读环境造成不可预测。
- `Theme` 是否需要 `Arc`？第一版建议不用，保持纯值 `Clone`；若后续引入动态扩展 slot 或大对象，再考虑 `Arc<Theme>`。
- 默认 style 从硬编码迁移到 theme slot 是否算 breaking？对 `element!` 用户影响较小，对直接构造 `Props::default()` 后读字段的用户有语义变化，需要在 OpenSpec 里标明。

## 相关文件

- `crates/ratatui-kit/src/components/context_provider.rs`：现有 context 注入容器，可作为 `ThemeProvider` 的实现基础。
- `crates/ratatui-kit/src/hooks/use_context.rs`：现有 context hook，可作为 `use_theme()` 的底层来源。
- `crates/ratatui-kit/src/components/{text,border,input,search_input,select,multi_select,table,modal,alert_modal,confirm_modal,shortcut_info_modal}.rs`：主题 slot 的首批接入面。
- `crates/ratatui-kit/src/render/harness.rs`：离屏 buffer 渲染测试，可断言样式。
- `dev-notes/knowledge/macros-and-props.md`：手写组件与 context-aware hook 边界、props 默认值约定。
- `dev-notes/knowledge/runtime-architecture.md`：渲染循环、响应式与 context 查找约束。
- `dev-notes/knowledge/hooks-and-state.md`：Hook 调用顺序与组件抽象边界。
