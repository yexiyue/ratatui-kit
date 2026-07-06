## ADDED Requirements

### Requirement: 三层生态结构

生态 MUST 分为三层:核心 `ratatui-kit`(主仓库,承载运行时与一等基础组件)、官方扩展(集中于 `ratatui-kit-contrib` monorepo,各组件为独立发布的 crate)、社区独立 crate。组件归属 MUST 按判据划分:运行时 / 通用基础能力归核心;官方维护的高质量扩展归 contrib;其余归社区。

#### Scenario: 判定组件归属

- **WHEN** 决定一个新组件放在哪一层
- **THEN** 按「运行时 / 通用基础 → 核心;官方维护的高质量扩展 → contrib;其余 → 社区」归位,而非默认合入主库

### Requirement: crate 命名前缀与 keyword

官方扩展 crate MUST 命名为 `ratatui-kit-<name>`;社区第三方 crate SHALL 采用同一 `ratatui-kit-<name>` 前缀。所有生态 crate MUST 携带 crates.io keyword `ratatui-kit`;官方维护的 crate MUST 在 description 中标注 `official`。

#### Scenario: 发布命名

- **WHEN** 发布一个表格组件 crate
- **THEN** 命名为 `ratatui-kit-<name>`,`keywords` 含 `ratatui-kit`

#### Scenario: 区分官方与社区

- **WHEN** 用户在 crates.io 检视一个 `ratatui-kit-*` crate
- **THEN** 官方维护的在 description 标 `official`,社区的不标,二者可区分

### Requirement: 发现机制

生态 MUST 提供 `awesome-ratatui-kit` 列表汇总可用组件;主库 README MUST 含「Ecosystem」段指向该列表与 keyword 检索方式。

#### Scenario: 用户查找可用组件

- **WHEN** 用户想找一个现成的组件
- **THEN** 可经 crates.io keyword `ratatui-kit` 或 `awesome-ratatui-kit` 列表发现

### Requirement: 发布流程复用

官方扩展 crate 的发布 MUST 复用主库「改版本 → 打 tag → CI 校验版本一致并 cargo publish + git-cliff 生成 CHANGELOG」流程。

#### Scenario: 发布官方扩展

- **WHEN** 官方扩展 crate 升版本并打 tag
- **THEN** CI 校验 `Cargo.toml` 版本与 tag 一致后 `cargo publish`,不一致则失败

### Requirement: 试点落地

本 change 的试点 MUST 为:PR #11 table 作为一等基础组件(feature 门控)合入主库核心;PR #12 markdown 生态迁出为独立 crate `ratatui-kit-markdown`(置于 contrib),且只依赖扩展 API 稳定面。

#### Scenario: table 归核心

- **WHEN** 处理 PR #11
- **THEN** table 以 feature 门控形式合入主库核心

#### Scenario: markdown 独立

- **WHEN** 处理 PR #12
- **THEN** markdown 生态迁出为 `ratatui-kit-markdown`,不合入主库,仅依赖公共扩展 API
