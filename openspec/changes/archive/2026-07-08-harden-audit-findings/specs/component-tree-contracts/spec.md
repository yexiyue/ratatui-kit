## ADDED Requirements

### Requirement: 透明布局空子树重置布局

透明布局组件(`#[component]` 生成、`set_transparent_layout(true)`)在某帧无子节点时 SHALL 把自身 `layout_style` 重置为 `LayoutStyle::default()`,MUST NOT 保留上一帧从子节点继承的旧值。

#### Scenario: 条件渲染为空时不粘连旧布局
- **WHEN** 一个透明布局组件本帧返回空(无子节点,如控制流无产出)
- **THEN** 其 `layout_style` 为默认值,而非沿用上一帧从首个子节点继承的布局

### Requirement: calc_children_areas 区域数等于子节点数

`Component::calc_children_areas` 返回的区域数量 SHALL 等于子节点数量;`draw` 中配对区域与子节点处 SHALL 以 `debug_assert` 校验该契约,并在 trait 文档中明确该约定。

#### Scenario: 区域数不匹配在 debug 期暴露
- **WHEN** debug 构建中某 `calc_children_areas` 实现返回的区域数与子节点数不符
- **THEN** `debug_assert` 失败,而非 `zip` 静默丢弃尾部子节点的绘制

### Requirement: poll_change 三路全部求值以注册 Waker

`InstantiatedComponent::poll_change` SHALL 对组件、子节点、hooks 三路全部求值(以注册各自的 Waker),MUST NOT 使用短路求值在某一路 `Ready` 后跳过其余路的 poll;该不变量 SHALL 以注释或统一辅助函数固定下来。

#### Scenario: 任一路变更都不丢唤醒
- **WHEN** 某一路返回 `Ready` 而其余路仍 `Pending`
- **THEN** 仍 `Pending` 的路在本次 poll 中注册了 Waker,后续其变更能正常唤醒重渲
