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

