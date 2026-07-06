## ADDED Requirements

### Requirement: 组件作者只依赖公共扩展 API

第三方组件 crate MUST 仅依赖 `ratatui-kit` 扩展 API 稳定面中的项,不得依赖标注为 internal 的实现项;需要 ratatui / crossterm 类型时 MUST 经 `ratatui_kit::ratatui` / `ratatui_kit::crossterm` re-export 获取,不单独声明 `ratatui` 依赖,以避免版本双开。

#### Scenario: 获取 ratatui 类型

- **WHEN** 组件需要 `Constraint`、`Style` 等 ratatui 类型
- **THEN** 经 `ratatui_kit::ratatui::...` 引用,`Cargo.toml` 不单独声明 `ratatui` 依赖

### Requirement: 透明布局约定

使用 `#[component]` 的函数组件作者 MUST 将布局属性(gap / flex_direction / width / height 等)写在返回的根元素上,因为函数组件是透明布局包装器、本身不占独立布局节点。

#### Scenario: 函数组件设置布局

- **WHEN** 作者希望自定义函数组件支持布局属性
- **THEN** 在返回的根 `element!` 元素上设置布局属性,而非期望包装器本身承载

### Requirement: feature 门控与最小默认

会引入额外依赖的组件能力 MUST 通过 Cargo feature 门控,可选依赖绑定到对应 feature,默认 feature 保持最小。

#### Scenario: 门控重依赖

- **WHEN** 组件依赖如语法高亮、正则等重量级 crate
- **THEN** 该依赖声明为 `optional = true` 并绑定到对应 feature,默认不启用

### Requirement: 面向使用者的运行时文案用英文

组件触发的、面向库使用者的 panic / expect / 错误文案 MUST 使用英文(国际化);源码注释可用中文。

#### Scenario: 运行时 panic

- **WHEN** 组件因误用触发面向使用者的 panic 或 expect
- **THEN** 其文案为英文

### Requirement: 编译即基线与版本区间声明

组件 crate MUST 保证其所有 example 与 doctest 能编译通过,并在 `Cargo.toml` 中以版本区间约束 `ratatui-kit`(匹配所依赖的扩展 API 稳定面版本)。

#### Scenario: CI 编译基线

- **WHEN** 组件 crate 的 CI 运行
- **THEN** 所有 example 与 doctest 编译通过

#### Scenario: 声明兼容版本区间

- **WHEN** 发布组件 crate
- **THEN** `Cargo.toml` 用版本区间(如 `ratatui-kit = ">=0.7, <0.8"`)约束依赖,而非无上界的开放版本

### Requirement: 提供 cargo-generate 起步模板

生态 MUST 提供 `ratatui-kit-component-template`,预置一个示例组件、一个自定义 hook、可运行的最小 example、fmt/clippy/test/doc CI 与「打 tag 发布」配置。

#### Scenario: 新作者起步

- **WHEN** 新作者用 `cargo generate` 该模板创建组件 crate
- **THEN** 得到开箱即用的骨架,`cargo build` 与 `cargo run --example` 直接成功,且 CI 与发布配置齐备
