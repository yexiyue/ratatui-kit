# component-from-props Specification

## Purpose
TBD - created by archiving change reduce-component-boilerplate. Update Purpose after archive.
## Requirements
### Requirement: 单一构造源

适用本约定的组件 SHALL 提供单一的 `from_props(props) -> Self`（或等价构造源）以从 props 派生自身存储状态；其字段镜像列表 MUST 只出现一处。

#### Scenario: 字段列表只写一次
- **WHEN** 一个规整组件需要从 props 镜像若干字段
- **THEN** 这些字段的赋值只在 `from_props` 中出现一次，`new` 与 `update` 不再各自重复该字段列表

### Requirement: new 与 update 行为一致

`new` 与 `update` 对「自身状态从 props 的派生」SHALL 产生一致结果；两者 MUST 共享同一构造源，使新增/删除/改名一个被镜像字段时无需改动两处。

#### Scenario: 改一个字段不漂移
- **WHEN** 给某规整组件的 props 增加一个被镜像的字段
- **THEN** 只需在 `from_props` 增加该字段，`new` 与 `update` 自动一致，无需分别修改

### Requirement: update 的布局与子节点收尾保持显式

迁移后，`update` 中「同步 `LayoutStyle`」与「`update_children`」的收尾 SHALL 保持显式调用，MUST NOT 被并入 `from_props`（`from_props` 只负责派生自身存储状态，不触及 updater）。

#### Scenario: 收尾职责不混入构造源
- **WHEN** 一个带布局与子节点的规整组件迁移到本约定
- **THEN** `from_props` 仅构造自身状态，`set_layout_style` 与 `update_children` 仍在 `update` 中显式调用

### Requirement: 定制组件豁免

`new`/`update` 含定制逻辑（条件分支、自管状态、自定义 `calc_children_areas`）的组件 SHALL 可豁免本约定、保持手写实现，且其行为 MUST NOT 被本变更改变。

#### Scenario: 定制组件不被强迁
- **WHEN** 组件（如 modal、scroll_view）的 `new`/`update` 含与单纯字段镜像不同的逻辑
- **THEN** 该组件保留手写 `new`/`update`，不纳入本次迁移，行为不变

