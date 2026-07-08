## Context

核心 `ratatui-kit` 0.10 已经拥有 always-on 主题协议:
`Palette` 是唯一颜色真源,每个组件通过 `FooTheme::from_palette(&Palette)` 派生样式,
`PaletteProvider` 负责向子树注入当前 palette。这个协议适合框架内置组件,但不应让核心包继续吸收
外部主题 catalog 依赖。

`ratatui-kit-contrib` 当前是官方扩展 monorepo,已有 `ratatui-kit-markdown`。该 crate 仍有多处
硬编码颜色:blockquote、code block、divider、markdown heading/list/table/rule、diff 和 inline
code/link 等。它需要像核心组件一样接入 `Palette`,否则 gallery 切换主题时 markdown 区域会和
core 组件脱节。

`ratatui-themes` 最新 `0.2.0` 提供 `ThemeName::all()` / `next()` / `prev()` /
`display_name()` / `slug()` / `palette()` 以及 `ThemePalette`。`ThemePalette` 字段为
`accent`、`secondary`、`bg`、`fg`、`muted`、`selection`、`error`、`warning`、
`success`、`info`,正好能有损但稳定地映射到 `ratatui_kit::Palette`。

## Goals / Non-Goals

**Goals:**

- 新增 `ratatui-kit-themes` 作为官方扩展 crate,只负责 `ratatui-themes` 到
  `ratatui_kit::Palette` 的转换。
- 为用户提供可直接组合 `PaletteProvider` 的 API,而不是引入第二套主题 Provider。
- 提供 gallery example,在同一屏展示 core 组件和 markdown 组件,并可切换主题和背景策略。
- 让 `ratatui-kit-markdown` 的默认样式跟随 `PaletteProvider`,同时保留 per-call 覆盖。
- 将 contrib workspace 的 `ratatui-kit` 依赖基线统一到 `>=0.10`,不再设置 `<0.11` 上限。

**Non-Goals:**

- 不集成 `ratatui-themekit`,不暴露其 builder、widget style 或 application theme 模型。
- 不修改核心 `ratatui-kit` 主题协议的数据结构或解析链。
- 不把 `ratatui-themes` 的 widget feature 作为主路径;gallery 用 ratatui-kit 组件自己展示。
- 不承诺 `ratatui-themes` 每个上游主题都拥有完美对比度;本 change 只提供确定映射和验证入口。

## Decisions

### D1. 主题适配放在 contrib,核心只保留协议

`ratatui-kit-themes` 输出 `ratatui_kit::Palette`,用户仍通过核心 `PaletteProvider` 使用主题:

```text
ratatui_themes::ThemeName
        │ palette()
        ▼
ratatui_themes::ThemePalette
        │ into_kit_palette()
        ▼
ratatui_kit::Palette
        │
        ▼
PaletteProvider
```

这修正旧规范中“外部适配器放核心 feature”的方向。核心包不应直接依赖每个生态主题 catalog;
官方维护的适配器更适合在 contrib 中随生态演进。

### D2. 公共 API 使用函数 + 本地 extension trait,不实现外部 From

由于 orphan rule,contrib crate 不能实现 `From<ratatui_themes::ThemePalette> for
ratatui_kit::Palette`。公共 API 采用普通函数和本地 trait:

- `pub use ratatui_themes::{Theme, ThemeName, ThemePalette};`
- `pub fn palette_from_name(name: ThemeName) -> Palette`
- `pub fn palette_from_theme_palette(source: ThemePalette) -> Palette`
- `pub trait IntoKitPalette { fn into_kit_palette(self) -> Palette; }`

trait 可实现给 `ThemeName`、`ratatui_themes::Theme`、`ThemePalette`,因为 trait 本地定义。

### D3. Palette 映射默认忠实使用 theme bg,另提供终端背景策略

默认转换应尊重 `ratatui-themes` 的完整主题背景:

```text
fg            <- fg
fg_dim        <- muted
bg            <- bg
surface       <- bg
overlay       <- bg
accent        <- accent
on_accent     <- 根据 selection/accent 亮度推导 Black/White
selection     <- selection
border        <- muted
border_active <- accent
success       <- success
warning       <- warning
error         <- error
info          <- info
placeholder   <- muted
```

另外提供 `terminal_background(palette)` 或等价 helper,把 `bg` / `surface` / `overlay` 清回
`Color::Reset`,用于希望底色继续跟随终端/VHS 主题的应用。gallery 通过 `b` 切换两种模式,
让用户直接看差别。

### D4. Markdown crate 以 contrib-local ComponentTheme 接入主题

`ratatui-kit-markdown` 不应依赖 `ratatui-kit-themes`;它只依赖核心 `ratatui-kit` 主题协议。
新增/整理 contrib-local 主题类型:

- `MarkdownTheme`: heading marker、list marker、inline code、link、table border 等。
- `CodeBlockTheme`: line number、plain code、border、language label。
- `BlockquoteTheme`: prefix bar、background。
- `DividerTheme`: rule style。
- `DiffTheme`: add/remove/unchanged/line number styles。

这些主题实现 `ComponentTheme::from_palette(&Palette)`,组件在 `update` / 函数组件体内读取
`use_component_theme::<T>()`。props 中显式样式仍按 core 语义 patch:
`None` 用主题、`Some(style)` 覆盖、`Some(Style::reset())` 清空。

Markdown parser 当前在解析阶段写入部分 inline `Span` 样式。为了让 inline code / link /
heading/list marker 也主题化,实现时可以选择最小 IR 调整:解析阶段保留 Markdown 语义,
渲染阶段根据 `MarkdownTheme` 生成 `Span`。如果一次重构过大,可先把块级组件主题化,
但最终任务完成前 inline 语义色也必须跟随主题。

### D5. Gallery 是主题压力测试,不是营销 demo

`ratatui-kit-themes` 的 example 采用一个“大通铺”界面:

- palette swatches: accent / secondary / selection / success / warning / error / info。
- core 组件: Text / Border / Select / Input / SearchInput / Table / TreeSelect / Modal-like preview。
- markdown 组件: Markdown preview、CodeBlock、Blockquote、Divider、Diff。
- controls: `t`/`T` 下一主题,`b` 切换背景策略,`q` 退出。

示例必须真实运行,用 package-local VHS tape 录制到 crate-local assets。它同时承担视觉 QA:
切换几个浅色/深色主题时,应能肉眼检查选中态、边框、markdown inline code/link/diff 是否可读。

### D6. Contrib 依赖基线改为 `ratatui-kit >=0.10`

`ratatui-kit-contrib` 是官方同步维护仓库,各 crate 可用:

```toml
ratatui-kit = ">=0.10"
```

不再写 `<0.11` 上限。这把兼容责任从 Cargo 版本约束转给官方同步维护流程:当核心 Extension API
发生不兼容变化时,contrib 同步更新并由 CI 发现问题。所有 contrib crate 仍不得直接依赖
`ratatui`;`ratatui`/`crossterm` 类型经 `ratatui_kit::ratatui` / `ratatui_kit::crossterm`
取得。

## Risks / Trade-offs

- 上游 `ratatui-themes` 使用 `ratatui` 0.30.x,而 `ratatui-kit` 也依赖 0.30.x;若未来两边
  minor 不一致,公共 API 可能出现 `ratatui::style::Color` 类型不统一。Mitigation:
  `ratatui-kit-themes` CI 必须编译实际转换代码;必要时在发布前调整依赖版本或转换边界。
- `>=0.10` 无上限可能让未来不兼容核心版本被 Cargo 选中。Mitigation:contrib 作为官方仓库,
  每次核心 breaking 前后同步跑 CI,README 明确其与核心最新 Extension API 同步维护。
- `on_accent` 推导是启发式,少数主题可能对比度不理想。Mitigation:gallery 作为人工视觉检查,
  并允许未来在 `ratatui-kit-themes` 中为特定主题覆盖映射。
- Markdown inline parser 主题化可能扩大改动面。Mitigation:任务拆分为块级组件主题化与 inline
  语义渲染两步,但最终验收要求两者都跟随 `Palette`。

## Migration Plan

1. 先更新 contrib workspace 依赖基线到 `>=0.10`,确认现有 markdown crate 仍编译。
2. 新增 `ratatui-kit-themes` 并实现 `ratatui-themes` 转换 API 与单元测试。
3. 将 `ratatui-kit-markdown` 接入主题协议,更新 props 默认值、theme 类型、README 与 examples。
4. 新增 gallery example + VHS recording,人工检查多个主题。
5. 跑 contrib workspace fmt / clippy / test / doc,再更新主仓 OpenSpec/docs。

## Open Questions

- 是否需要在 `ratatui-kit-themes` 首版提供少量主题特例映射(例如浅色主题的 `selection` /
  `on_accent`),还是先完全机械映射并依赖 gallery 暴露问题?
