## Why

手写 `impl Component` 的组件（`border`/`positioned`/`view`/`fragment` 等）在 `new` 与 `update` 中重复同一套「从 props 镜像字段」的赋值——同一份字段列表写两遍，改字段时易只改一处导致 `new`/`update` 行为漂移。这是跨多个组件、易出 bug 的可维护性债。

## What Changes

- 为「镜像 props 字段」的规整组件引入**单一构造源**约定：组件实现一个 `fn from_props(props) -> Self`，字段列表只写一次；`new` 与 `update` 都由它派生自身状态，不再各写一遍字段。
- 把符合该模式的组件（如 `border`/`positioned`/`view`/`fragment`）迁移到该约定。
- `update` 中「同步 `LayoutStyle` + `update_children`」的固定收尾保持显式（属布局/子节点职责，不混入 `from_props`）。
- 有定制 `new`/`update` 逻辑的组件（`modal`/`scroll_view`/`context_provider`）**不强迁**，保持手写。
- `draw` / `calc_children_areas` 仍由组件自行实现（与本变更无关）。

## Capabilities

### New Capabilities
- `component-from-props`: 规整组件如何从 props 派生自身状态的约定——单一 `from_props` 构造源、`new`/`update` 不重复字段列表、`update` 的布局/子节点收尾职责划分、以及哪些组件适用/豁免。

### Modified Capabilities
<!-- 无：openspec/specs/ 当前为空。 -->

## Impact

- **代码**：迁移 `packages/ratatui-kit/src/components/` 下符合模式的组件（`border`/`positioned`/`view`/`fragment` 等）的 `new`/`update`。
- **公开 API**：不变（`from_props` 为组件私有；`Component` trait 不变；DSL 用法不变）。
- **依赖**：无新增。
- **风险**：低-中。纯内部重构，行为须与手写等价；无单元测试仓库，回归靠 `examples/`（counter/modal/list/router 等）编译并运行表现不变。
- **非破坏性**。
