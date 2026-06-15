## ADDED Requirements

### Requirement: 动态路由尾部静态段按段边界匹配

对参数段后还有静态尾段的动态路由(如 `/users/:id/edit`),`Outlet` 匹配成功后 SHALL 校验剩余路径为空或以 `/` 起始(与静态段边界语义一致),MUST NOT 让尾段以前缀方式跨段误匹配。等价地,其匹配正则在尾部 SHALL 锚定段边界(`$` 或 `/`)。

#### Scenario: 尾段不跨段误匹配
- **WHEN** 路由为 `/users/:id/edit`,当前路径为 `/users/42/edit-more`
- **THEN** 该路由不匹配(`edit` 之后的 `-more` 不构成新段),不会误匹配并捕获错误剩余串

#### Scenario: 合法精确与嵌套仍匹配
- **WHEN** 路由为 `/users/:id/edit`,当前路径为 `/users/42/edit` 或 `/users/42/edit/sub`
- **THEN** 该路由匹配,`id = 42`,剩余路径为空或 `/sub`

### Requirement: Outlet 无匹配时优雅兜底

当无任何路由匹配当前路径、且不存在根路由兜底时,`Outlet` SHALL 渲染空内容(或可配置的 NotFound 兜底组件),MUST NOT `expect`/panic 拖垮整个应用。

#### Scenario: 导航到未声明路径不崩溃
- **WHEN** 用户导航到任意未声明、且无根路由兜底的路径(如拼写错误)
- **THEN** `Outlet` 渲染空/NotFound 内容,应用继续运行、终端不被破坏

### Requirement: 路由优先级语义明确

路由匹配按 `routes!` 声明顺序取首个匹配项;该"声明顺序即优先级"语义 SHALL 在文档中明确(静态路由应声明在同前缀的动态路由之前,以免被遮蔽)。

#### Scenario: 声明顺序决定匹配
- **WHEN** 同时声明 `/users/new` 与 `/users/:id`,且 `/users/new` 声明在前
- **THEN** 导航 `/users/new` 命中静态路由 `/users/new`,而非被 `/users/:id` 遮蔽

### Requirement: 不提供静默丢弃的可变路由句柄

路由上下文 MUST NOT 暴露"可变但修改被静默丢弃"的 API。原 `use_route_mut`(返回 `Context::owned` 临时克隆的可变引用、改动每帧被丢弃且不回写路由表)SHALL 被移除。

#### Scenario: 误导性可变路由访问被移除
- **WHEN** 用户需要修改与路由相关的持久状态
- **THEN** 不存在 `use_route_mut` 这种"看似可变实则丢弃"的 API;改用 `use_state`/`AtomState` 或 `use_route_state` + `push_with_state`
