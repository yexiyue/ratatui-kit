## ADDED Requirements

### Requirement: ratatui-themes 适配 crate

官方 contrib MUST 提供 `ratatui-kit-themes` crate,用于把 `ratatui-themes` 的主题 catalog
转换为 `ratatui_kit::Palette`。该 crate MUST 只集成 `ratatui-themes`,MUST NOT 引入
`ratatui-themekit`。其公共主路径 MUST 输出 `ratatui_kit::Palette`,MUST NOT 另行提供替代
`PaletteProvider` 的主题 Provider 或绕开 `ComponentTheme` 解析链。

#### Scenario: ThemeName 转为 Palette

- **WHEN** 用户选择任一 `ratatui_themes::ThemeName`
- **THEN** `ratatui-kit-themes` 能将其转换为 `ratatui_kit::Palette`
- **THEN** 用户可把该 `Palette` 传给核心 `PaletteProvider`

#### Scenario: 不接 ratatui-themekit

- **WHEN** 检视 `ratatui-kit-themes` 的依赖与公共 API
- **THEN** 不存在 `ratatui-themekit` 依赖、feature、re-export 或 builder/widget style 入口

### Requirement: 适配 API 避免 orphan rule

`ratatui-kit-themes` MUST 通过本地函数和本地 extension trait 暴露转换能力。由于 orphan rule,
该 crate MUST NOT 尝试实现 `From<ratatui_themes::ThemePalette> for ratatui_kit::Palette`。
转换 API MUST 至少覆盖 `ThemeName` 与 `ThemePalette`;MAY re-export `ratatui_themes`
的 `ThemeName`、`Theme`、`ThemePalette` 以降低用户 import 成本。

#### Scenario: 本地 trait 可用于上游类型

- **WHEN** 用户导入 `ratatui_kit_themes::IntoKitPalette`
- **THEN** 可对 `ratatui_themes::ThemeName` 调用 `into_kit_palette()`
- **THEN** 编译不触发 orphan rule 冲突

#### Scenario: 普通函数可用于显式转换

- **WHEN** 用户不想导入 extension trait
- **THEN** 可调用普通函数把 `ThemeName` 或 `ThemePalette` 转为 `Palette`

### Requirement: ThemePalette 到 Palette 的确定映射

`ratatui-kit-themes` MUST 对 `ratatui_themes::ThemePalette` 的每个语义色执行确定映射:
`fg <- fg`,`fg_dim <- muted`,`bg <- bg`,`surface <- bg`,`overlay <- bg`,
`accent <- accent`,`selection <- selection`,`border <- muted`,`border_active <- accent`,
`success <- success`,`warning <- warning`,`error <- error`,`info <- info`,
`placeholder <- muted`。`on_accent` MUST 通过可重复的亮度/对比度规则从 `accent` 或
`selection` 推导为可读前景色。

#### Scenario: 全部 ratatui-themes 主题可转换

- **WHEN** 遍历 `ThemeName::all()` 并逐个转换为 `Palette`
- **THEN** 每个转换都成功
- **THEN** 输出 `Palette` 的语义字段均有确定颜色

#### Scenario: on_accent 可读

- **WHEN** 转换结果用于 Select/Table/TreeSelect 等选中态
- **THEN** `on_accent` 与 `selection` 或 `accent` 形成可读前景/背景组合

### Requirement: 背景策略可切换

默认转换 MUST 忠实使用 `ratatui-themes` 的 `bg` 作为 `Palette.bg` / `surface` / `overlay`。
`ratatui-kit-themes` MUST 同时提供 helper,可将已转换 `Palette` 的背景相关字段清回
`Color::Reset`,以便应用继续使用终端背景。

#### Scenario: 默认使用主题背景

- **WHEN** 用户直接把 `ThemeName` 转为 `Palette`
- **THEN** `Palette.bg` 取自 `ThemePalette.bg`

#### Scenario: 终端背景模式

- **WHEN** 用户对转换结果应用 terminal-background helper
- **THEN** `Palette.bg`、`Palette.surface`、`Palette.overlay` 使用 `Color::Reset`
- **THEN** 其它语义色保持来自原主题

### Requirement: Gallery example 覆盖 core 与 markdown 组件

`ratatui-kit-themes` MUST 提供真实运行的 gallery example。该 example MUST 用
`PaletteProvider` 驱动一屏“大通铺”组件预览,并同时展示核心组件、`ratatui-kit-markdown`
组件和 palette swatches。example MUST 支持按键切换 `ratatui-themes` 主题、切换背景策略、
退出程序。

#### Scenario: 切换主题更新整屏预览

- **WHEN** 用户在 gallery example 中按主题切换键
- **THEN** core 组件和 markdown 组件在下一帧一起换色
- **THEN** 当前主题名或 slug 在界面中更新

#### Scenario: 大通铺暴露关键状态

- **WHEN** gallery example 首帧稳定后显示
- **THEN** 屏幕包含 palette swatches、至少一个选中态组件、一个状态色组件、一个表格/树或列表预览、
  以及 markdown/code/diff/blockquote/divider 预览

### Requirement: Gallery 录制可复现

`ratatui-kit-themes` MUST 按 contrib 仓库约定提供 package-local VHS tape 和 GIF asset。
tape MUST 从真实 example 录制,MUST NOT 使用手绘/伪造终端图。录制流程 MUST 避免把编译日志、
shell prompt 或多余空白录入最终 GIF。

#### Scenario: 录制资产存在且可重建

- **WHEN** 从 contrib 仓库根运行 gallery tape
- **THEN** 生成 crate-local gallery GIF asset
- **THEN** GIF 展示真实运行的 gallery example

### Requirement: ratatui-kit-markdown 接入 Palette 主题

`ratatui-kit-markdown` MUST 使用核心主题协议解析默认样式。Markdown、CodeBlock、Blockquote、
Divider、Diff 等组件/渲染路径 MUST 定义或使用 contrib-local `*Theme`,并通过
`ComponentTheme::from_palette(&Palette)` 从当前 `Palette` 派生默认样式。组件样式 props
MUST 保持 `Option<Style>` 语义:`None` 用主题、`Some(style)` patch 主题、
`Some(Style::reset())` 清空。

#### Scenario: Markdown 组件跟随 PaletteProvider

- **WHEN** `Markdown`、`CodeBlock`、`Blockquote`、`Divider` 或 `Diff` 位于某个
  `PaletteProvider` 子树内
- **THEN** 其默认颜色来自当前 `Palette`
- **THEN** 切换 `Palette` 后下一帧默认颜色随之更新

#### Scenario: 显式样式覆盖仍生效

- **WHEN** 用户向 markdown 组件传入某个 `Some(Style)` 样式 prop
- **THEN** 该样式在主题解析结果之上 patch
- **THEN** 未覆盖的样式字段继续保留主题值

### Requirement: Markdown inline 语义色跟随主题

`ratatui-kit-markdown` 的 Markdown 渲染 MUST 让 heading marker、list marker、inline code、
link、table border/rule 等 inline/block 语义色跟随 `MarkdownTheme` 或相关组件主题。
解析阶段 MUST NOT 把这些语义永久固化为不可被 `Palette` 覆盖的硬编码颜色。

#### Scenario: inline code 跟随主题

- **WHEN** Markdown 内容包含 inline code
- **THEN** inline code 的前景/背景样式来自当前 markdown 主题

#### Scenario: link 跟随主题

- **WHEN** Markdown 内容包含链接
- **THEN** 链接文本样式来自当前 markdown 主题的信息/强调色

### Requirement: Contrib 主题验证

本 change MUST 提供自动测试和人工视觉验证路径。自动测试至少 MUST 覆盖全部
`ThemeName::all()` 可转换、背景策略 helper、markdown 主题默认值来自 `Palette`、
per-call override 语义。人工验证 MUST 使用 gallery example 与 VHS/Screenshot 检查
浅色和深色主题下的可读性。

#### Scenario: 全主题转换测试

- **WHEN** 运行 contrib workspace 测试
- **THEN** `ThemeName::all()` 中每个主题都能转为 `Palette`

#### Scenario: 视觉验证覆盖浅色与深色

- **WHEN** 使用 gallery tape 或验证截图检查主题
- **THEN** 至少覆盖一个浅色主题和一个深色主题
- **THEN** 选中态、边框、markdown inline code/link/diff 均可读
