## MODIFIED Requirements

### Requirement: 扩展 API 稳定面文档化

框架 MUST 提供一份文档,枚举对第三方组件作者承诺遵守 semver 的公共项(扩展 API 稳定面),并将非承诺的内部实现项标注为 internal 或 `#[doc(hidden)]`。稳定面至少 MUST 覆盖:组件契约(`Component`、`ComponentUpdater`、`ComponentDrawer`、`Element`、`AnyElement`、`ElementKey`、`NoProps`)、过程宏、Hooks(`Hooks`、`Hook`、`use_hook` 及内置 hooks)、状态与布局(`State`、`LayoutStyle`)、**主题协议(`Palette`、`ComponentTheme`、`use_palette`、`use_component_theme`、`PaletteProvider`、`ThemeOverride`)**、re-export(`ratatui`、`crossterm`)。

#### Scenario: 作者查阅稳定面

- **WHEN** 组件作者阅读扩展 API 文档
- **THEN** 能明确区分哪些项承诺稳定可依赖、哪些是内部实现不应依赖

#### Scenario: 内部项标注

- **WHEN** 检视 `ComponentHelperExt`、`AnyProps` 等内部实现项
- **THEN** 它们被标注为 internal(文档说明或 `#[doc(hidden)]`),不列入稳定面清单

#### Scenario: 第三方组件接入主题

- **WHEN** 一个第三方组件作者依据稳定面文档,为自己的组件读取 `use_palette()` 并实现 `from_palette` 派生
- **THEN** 文档明确 `Palette` / `ComponentTheme` / 主题 hooks 属承诺稳定面,作者可安全依赖以接入主题系统
