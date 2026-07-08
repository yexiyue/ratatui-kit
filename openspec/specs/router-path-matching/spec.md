# router-path-matching Specification

## Purpose
TBD - created by archiving change cache-router-regex. Update Purpose after archive.
## Requirements
### Requirement: 静态路径按段边界匹配

`Outlet` SHALL 将当前路径与无动态参数的路由按「路径段边界」匹配：当前路径以该路由路径为前缀，且前缀之后为空（精确匹配）或紧跟 `/`（继续嵌套匹配）。前缀落在段中间的情况 MUST NOT 视为匹配。

#### Scenario: 精确匹配
- **WHEN** 当前路径为 `/book-source` 且存在路由 `/book-source`
- **THEN** 该路由匹配，剩余路径为空

#### Scenario: 段边界保护，避免误匹配
- **WHEN** 当前路径为 `/book-source-login` 且同时存在路由 `/book-source` 与 `/book-source-login`
- **THEN** `/book-source` 不匹配（剩余 `-login` 不是新段），`/book-source-login` 匹配

### Requirement: 动态参数提取

对路径含 `/:name` 段的路由，`Outlet` SHALL 把每个 `/:name` 段匹配为单个路径段（不跨 `/`），并将捕获到的值以 `name` 为键写入路由上下文的参数表。

#### Scenario: 提取命名参数
- **WHEN** 路由路径为 `/users/:id` 且当前路径为 `/users/42`
- **THEN** 该路由匹配，参数表包含 `id = "42"`

#### Scenario: 动态段不跨越路径分隔符
- **WHEN** 路由路径为 `/users/:id` 且当前路径为 `/users/42/profile`
- **THEN** `:id` 仅捕获 `42`，剩余路径为 `/profile` 供嵌套匹配

### Requirement: 匹配正则每个路由至多编译一次

含动态参数的路由，其用于匹配/提取的正则表达式 SHALL 在该路由（或路由表）的生命周期内至多编译一次，并在后续匹配中复用。`Outlet` 在每次渲染时 MUST NOT 重新编译该正则。

#### Scenario: 渲染不触发重新编译
- **WHEN** 同一个含动态参数的路由在应用生命周期内被匹配多次（多次 `Outlet` 渲染）
- **THEN** 其匹配正则只被编译一次，后续匹配复用同一已编译正则

### Requirement: 无动态参数的路由不付出正则代价

不含 `/:` 段的路由 SHALL 仅走字符串段边界匹配，MUST NOT 为其编译或持有任何正则。

#### Scenario: 静态路由零正则
- **WHEN** 路由路径为 `/settings`（无 `/:`）
- **THEN** 该路由的匹配过程不编译也不使用任何正则

### Requirement: 非法路由路径在构建期暴露

若某路由路径无法构成合法正则，该错误 SHALL 在构建路由表时暴露，而非延迟到 `Outlet` 渲染时才 panic。

#### Scenario: 非法路径尽早报错
- **WHEN** 一个含动态参数的路由路径无法编译为合法正则
- **THEN** 错误在路由表构建阶段被报告，且不会在每次渲染时重复触发

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

