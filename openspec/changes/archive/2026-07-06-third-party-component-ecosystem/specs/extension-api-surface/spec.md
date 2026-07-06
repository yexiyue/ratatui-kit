## ADDED Requirements

### Requirement: 过程宏在外部 crate 中可编译(宏 hygiene)

所有导出的过程宏(`#[component]`、`element!`、`#[derive(Props)]`、`#[with_layout_style]`、`routes!`)展开时 MUST 只引用绝对路径 `::ratatui_kit::...`(需要 ratatui / crossterm 类型时经 `::ratatui_kit::ratatui` / `::ratatui_kit::crossterm` re-export 转发),不得生成任何依赖「调用方作用域内存在裸 `ratatui` / `crossterm` crate 名」的代码。

#### Scenario: 外部 crate 使用 #[with_layout_style]

- **WHEN** 一个仅依赖 `ratatui-kit`(不直接依赖 `ratatui`)的外部 crate 对具名字段结构体应用 `#[with_layout_style]`
- **THEN** 该 crate 编译通过,注入的 `margin`/`offset`/`width`/`height`/`flex_direction`/`justify_content` 字段类型全部解析到 `::ratatui_kit::ratatui::layout::*`

#### Scenario: 外部 crate 组合使用全部宏

- **WHEN** 外部 crate 用 `#[component]` + `element!` + `#[derive(Props)]` 定义一个组件
- **THEN** 编译通过,不出现「cannot find module or crate」类路径错误

### Requirement: 扩展 API 稳定面文档化

框架 MUST 提供一份文档,枚举对第三方组件作者承诺遵守 semver 的公共项(扩展 API 稳定面),并将非承诺的内部实现项标注为 internal 或 `#[doc(hidden)]`。稳定面至少 MUST 覆盖:组件契约(`Component`、`ComponentUpdater`、`ComponentDrawer`、`Element`、`AnyElement`、`ElementKey`、`NoProps`)、过程宏、Hooks(`Hooks`、`Hook`、`use_hook` 及内置 hooks)、状态与布局(`State`、`LayoutStyle`)、re-export(`ratatui`、`crossterm`)。

#### Scenario: 作者查阅稳定面

- **WHEN** 组件作者阅读扩展 API 文档
- **THEN** 能明确区分哪些项承诺稳定可依赖、哪些是内部实现不应依赖

#### Scenario: 内部项标注

- **WHEN** 检视 `ComponentHelperExt`、`AnyProps` 等内部实现项
- **THEN** 它们被标注为 internal(文档说明或 `#[doc(hidden)]`),不列入稳定面清单

### Requirement: 稳定面变更遵守 semver

在 `0.x` 阶段,移除或不兼容地修改稳定面中的项 MUST 伴随 minor 版本号提升并记入 CHANGELOG;纯新增项可随 patch 版本发布。

#### Scenario: 收窄稳定面

- **WHEN** 需要移除或隐藏一个已列入稳定面的公共项
- **THEN** 该变更通过独立 change 提出、标注 BREAKING、并提升 minor 版本号
