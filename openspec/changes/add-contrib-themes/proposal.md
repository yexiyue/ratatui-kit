## Why

主题系统已经在核心 `ratatui-kit` 落地,但用户仍需要一个低成本入口来使用现成主题 catalog
(如 `ratatui-themes` 提供的 Catppuccin / Nord / Tokyo Night 等)。同时官方扩展
`ratatui-kit-markdown` 仍保留大量硬编码默认色,无法跟随 `PaletteProvider` 一起换肤。

现在适合把边界定清:核心包只承载 `Palette` / `ComponentTheme` 协议,官方 contrib 负责主题预设、
外部 catalog 适配和扩展组件主题化。

## What Changes

- 新增官方扩展 crate `ratatui-kit-themes`,只集成 `ratatui-themes`,提供
  `ratatui_themes::ThemeName` / theme palette 到 `ratatui_kit::Palette` 的转换。
- 新增主题 gallery example:以“大通铺”方式同时展示 core 组件、markdown 组件、palette swatches,
  并支持键盘切换主题和背景策略,用于人工检查主题映射质量。
- 修改 `ratatui-kit-markdown`:接入核心主题协议,为 Markdown / CodeBlock / Blockquote /
  Divider / Diff 等暴露 contrib-local `*Theme` 并从 `Palette` 派生默认样式。
- 保留 per-call override 语义:样式 props 继续为 `Option<Style>`,其中 `None` 用主题、
  `Some(style)` patch 主题、`Some(Style::reset())` 清空。
- 更新 `ratatui-kit-contrib` workspace 依赖基线:所有官方扩展 crate 依赖
  `ratatui-kit = ">=0.10"` 且不再设置 `<0.11` 上限,由 contrib 与核心同步维护承担兼容责任。
- 明确不集成 `ratatui-themekit`:本 change 不引入其依赖、不暴露其 builder / widget style
  系统,避免在 ratatui-kit 主题协议之外形成第二套主题模型。
- 修改核心主题系统规范:外部主题 catalog 适配器和预设主题包应位于 `ratatui-kit-contrib`,
  而不是核心 crate feature。

## Capabilities

### New Capabilities

- `contrib-themes`: 官方 contrib 主题扩展能力,覆盖 `ratatui-kit-themes` 的
  `ratatui-themes` 适配、Palette 映射策略、gallery example、VHS 录制与
  `ratatui-kit-markdown` 的主题系统接入。

### Modified Capabilities

- `theme-system`: 更改外部主题适配器归属边界:核心 `ratatui-kit` 继续 only-own
  `Palette` / `ComponentTheme` 协议,预设主题和 `ratatui-themes` 适配器归
  `ratatui-kit-contrib`。
- `ecosystem-conventions`: 更新官方扩展 crate 的依赖基线与发布约定,要求 contrib
  crate 基于 `ratatui-kit >=0.10` 并由官方同步维护,不再在版本要求里人为写 `<0.11`
  上限。

## Impact

- `ratatui-kit-contrib/Cargo.toml`: workspace member 增加 `crates/ratatui-kit-themes`,
  workspace dependency 改为 `ratatui-kit = ">=0.10"`。
- `ratatui-kit-contrib/crates/ratatui-kit-themes/`: 新 crate、README、examples、VHS tape、
  recording asset、tests。
- `ratatui-kit-contrib/crates/ratatui-kit-markdown/`: Cargo dependency baseline 更新到
  `>=0.10`;组件 props/default/style 解析改为主题驱动;文档和 example 更新。
- `ratatui-kit` 主仓 docs/specs: 更新主题系统和生态约定的 OpenSpec contract。
- 测试与验证: contrib workspace 的 fmt / clippy / test / doc 全绿;gallery example
  真实运行并用 VHS 录制;人工检查多个 `ratatui-themes` 主题下 core + markdown
  组件对比度和默认观感。
