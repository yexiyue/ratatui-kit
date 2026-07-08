## ADDED Requirements

### Requirement: 官方扩展依赖核心基线

官方 `ratatui-kit-contrib` crate MUST 以 `ratatui-kit >=0.10` 作为核心依赖基线,并由官方同步维护
承担与最新 Extension API 的兼容责任。官方扩展 crate MUST NOT 在 `ratatui-kit` 依赖要求中人为写
`<0.11` 上限。官方扩展 crate 仍 MUST 通过 `ratatui_kit::ratatui` /
`ratatui_kit::crossterm` 取得 re-export 类型,MUST NOT 直接依赖 `ratatui` 或 `crossterm`
作为公共组件类型来源。

#### Scenario: 新 contrib crate 使用 0.10 基线

- **WHEN** 新增一个官方 contrib crate
- **THEN** 其 `Cargo.toml` 中 `ratatui-kit` 依赖满足 `>=0.10`
- **THEN** 不包含 `<0.11` 上限

#### Scenario: 现有 contrib crate 更新基线

- **WHEN** 更新已有官方 contrib crate
- **THEN** 其 `ratatui-kit` 依赖从旧的 `>=0.9, <0.10` 策略迁移到 `>=0.10`

#### Scenario: 扩展类型来源仍走 re-export

- **WHEN** 检视官方 contrib crate 源码和依赖
- **THEN** 组件公共 API 中使用的 ratatui/crossterm 类型来自 `ratatui_kit` re-export
- **THEN** crate 不因组件 API 直接依赖一份独立的 `ratatui`
