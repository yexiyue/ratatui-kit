## Why

`Outlet` 每次渲染时，会对每个含动态参数（`/:`）的路由调用 `regex::Regex::new(...)` 重新编译匹配正则（`components/router/outlet.rs:43`，位于 `routes.iter_mut().find(...)` 闭包内）。但该正则仅由静态的 `Route.path` 决定、在路由表生命周期内恒定不变。正则编译的成本远高于一次字符串匹配，在路由型应用中属于每次渲染都付出的热路径浪费。

## What Changes

- 为每个 `Route` 的动态参数匹配正则引入**一次性编译 + 复用**（惰性缓存或在构建路由表时预编译）。
- `Outlet` 的路径匹配改为使用缓存的正则，渲染时不再 `Regex::new`。
- 含**非法正则**的路由路径，其报错从「每次渲染时 panic」前移到**路由表构建时**暴露（更早、更可定位）。
- 无动态参数（不含 `/:`）的路由继续走纯字符串「段边界」匹配，**不付出任何正则代价**。
- 不改变 `routes!` 宏的公开 DSL 用法；不改变现有「按段边界匹配 + 命名参数提取」的匹配语义。

## Capabilities

### New Capabilities
- `router-path-matching`: `Outlet` 如何把当前路径匹配到路由——静态路径的段边界匹配、动态 `/:` 段的命名参数提取、以及匹配正则「每个路由只编译一次并复用」的性能要求。

### Modified Capabilities
<!-- 无：openspec/specs/ 当前为空，无既有 spec 的需求被修改。 -->

## Impact

- **代码**：`packages/ratatui-kit/src/components/router/outlet.rs`（匹配逻辑改用缓存正则）、`packages/ratatui-kit/src/components/router/mod.rs`（`Route`/`Routes` 定义，新增编译后正则的缓存字段/构造）；可能涉及 `packages/ratatui-kit-macros/src/router.rs`（`routes!` 宏构造路由的位置）。
- **特性门控**：`router` feature。
- **类型约束**：`Route`/`Routes` 须保持 `Send + Sync`（props 要求）；缓存载体需满足该约束（如 `OnceLock`，或在构建期就编译好直接存 `Regex`）。
- **依赖**：复用既有 `regex`（已是 `router` feature 依赖），不新增依赖。
- **公开 API / DSL**：不变（纯内部性能优化，非破坏性）。
- **风险**：低。无单元测试仓库，回归依赖 `examples/router.rs`、`store.rs` 能编译并运行匹配正常。
