## MODIFIED Requirements

### Requirement: 主题协议 always-on 与特性门控边界

主题协议本体 MUST 在核心 crate always-on(被未门控组件消费,无法门控)且不引入新运行时依赖。
该协议本体包括 `Palette`、`ComponentTheme`、各 `*Theme`、`PaletteProvider` /
`ThemeOverride`、`use_palette` / `use_component_theme`。核心 crate MUST 只承载主题协议和内置组件主题,
MUST NOT 直接依赖外部主题 catalog crate。预设主题包、`ratatui-themes` 适配器和主题 gallery
MUST 置于官方 `ratatui-kit-contrib` 扩展 crate 中。核心 crate MAY 提供 `Palette` 的
serde 支持作为 feature,但 MUST NOT 因外部主题适配器引入运行时依赖。

#### Scenario: 默认特性下主题可用

- **WHEN** 一个未开启任何 feature 的下游依赖 ratatui-kit
- **THEN** `Palette` / `use_palette` / `PaletteProvider` 等协议项可直接使用

#### Scenario: 适配器不进入核心 crate

- **WHEN** 用户只依赖核心 `ratatui-kit`
- **THEN** dependency graph 中不包含 `ratatui-themes`
- **THEN** 核心 crate 不暴露 `ratatui-themes` 相关转换 API

#### Scenario: 适配器由 contrib 提供

- **WHEN** 用户需要使用 `ratatui-themes` 主题 catalog
- **THEN** 用户通过官方 contrib crate 获取 `ratatui-themes` 到 `Palette` 的转换能力
       并继续用核心 `PaletteProvider` 注入主题
