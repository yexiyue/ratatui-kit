## Context

`Outlet`（`components/router/outlet.rs`）在 `routes.iter_mut().find(...)` 闭包内，对每个含 `/:` 的路由：把 `Route.path` 按 `/` 切分、将 `:name` 段替换为 `(?<name>[^/]+)`、`join` 成正则字符串，再 `regex::Regex::new(...).expect(...)` 编译。该逻辑每次 `Outlet` 渲染都重跑一遍。

两个约束影响方案：

1. **`Route::borrow()` 每帧浅拷贝**（`router/mod.rs:21`：`Route { path: self.path.clone(), component, children }`）。任何缓存若是普通字段，会在 `borrow()` 时丢失或被重建。缓存载体必须能跨 `borrow()` **廉价共享**。
2. **`routes!` 宏生成 `Route { .. }` 结构体字面量**（`macros/src/router.rs`，字段 `path/component/children` 均为 `pub`）。新增字段会影响字面量构造方式。
3. `Route`/`Routes` 已 `unsafe impl Send + Sync`（为 `AnyElement`）；新增缓存须自身满足 `Send + Sync`。

## Goals / Non-Goals

**Goals:**
- 每个含动态参数的路由，其匹配正则在生命周期内至多编译一次，跨帧/跨 `borrow()` 复用。
- 非法路由路径的报错从「每次渲染 panic」前移到「路由表构建时」。
- 无动态参数的路由不编译、不持有任何正则。
- 不改 `routes!` 的公开 DSL；不改既有匹配语义。

**Non-Goals:**
- 不重写整体路由/匹配算法（段边界匹配、参数提取逻辑保持不变）。
- 不引入新依赖（复用既有 `regex`）。
- 不优化路由匹配的其它分配（如 `path.clone()`、`params` 的 String 分配）——超出本次范围。

## Decisions

### 决策 1：预编译（eager）而非惰性（lazy）

`Route` 新增私有字段 `matcher: Option<Arc<Regex>>`：含 `/:` 的路由在**构造时**编译并存 `Some(Arc<Regex>)`，静态路由存 `None`。

- **为什么 Arc**：`Route::borrow()` 每帧执行，`matcher: self.matcher.clone()` 即 `Arc::clone`（廉价、共享同一已编译正则），满足约束 1。直接存 `Regex` 虽 `Clone`，但 `Arc<Regex>` 让「克隆廉价且共享」成为显式保证。
- **为什么 eager 而非 lazy**：eager 在构造期即可暴露非法路径（满足 spec「构建期暴露」要求），且匹配热路径无需 `OnceLock` 的一次性初始化分支。代价是启动时编译所有动态路由（含从未访问的嵌套子路由）——路由数量级很小，可忽略。
- **备选（lazy）**：`matcher: Arc<OnceLock<Regex>>`，首次匹配时编译。优点是改动更小、按需编译；缺点是非法路径要到首次渲染该路由时才 panic（弱于构建期暴露），且每次匹配多一次 `OnceLock` 分支。**已否决**，因 eager 语义更干净且满足 spec。

### 决策 2：构造经 `Route::new(...)`，正则字符串构建逻辑随之下沉

新增 `pub fn Route::new(path, component, children) -> Route`，内部：判断 `path.contains("/:")` → 构建正则字符串（现 outlet.rs 内的 split/map/join 逻辑迁此）→ `Regex::new` 编译并包 `Arc`；否则 `matcher = None`。

- **为什么改构造方式**：`matcher` 设为**私有**字段后，外部（用户 crate 中由 `routes!` 展开的字面量）无法再用 `Route { .. }` 字面量构造。故 `routes!` 宏的 `ToTokens` 从生成结构体字面量改为生成 `Route::new(#path.to_string(), <component>, <children>.into())`。这是 `macros/src/router.rs` 一处 `quote!` 改动。
- `Route::borrow()`（crate 内）可继续用字面量（能访问私有 `matcher`），`matcher: self.matcher.clone()`。
- **非法正则处理**：路径是 `routes!` 里的静态字面量（开发者可控），非法即开发者 bug；`Route::new` 在构造期 `expect`/panic，比现状「每次渲染 panic」严格更优。保留 `expect` 简单直接，不引入 `Result` 让 `routes!` 复杂化。

### 决策 3：`Outlet` 改用 `route.matcher`

`Outlet` 匹配逻辑删去正则字符串构建与 `Regex::new`，改为：`if let Some(re) = &r.matcher { /* find / captures 提取参数 */ } else { /* 静态段边界匹配 */ }`。判断「是否动态」从 `path.contains("/:")` 改为 `matcher.is_some()`（等价且更省一次扫描）。

## Risks / Trade-offs

- **[启动期编译所有动态路由正则]** → 路由数量级小（典型 < 数十），编译成本一次性且远低于每帧重复编译；可接受。
- **[`Route` 字段从全 `pub` 字面量构造改为 `new()`]** → 仅 `routes!` 宏与 crate 内 `borrow()` 构造 `Route`；改宏一处、`borrow()` 一处即可，无其它构造点。若外部用户曾手写 `Route { .. }` 字面量（未在文档/示例中出现），属潜在 breaking——评估为极低概率，且可在 CHANGELOG 注明。
- **[正则字符串构建逻辑迁移]** → 逐字迁移、语义不变；回归靠 `examples/router.rs`（含 `/`、嵌套、静态路由）编译并运行匹配正常。

## Migration Plan

1. `Route` 加私有 `matcher: Option<Arc<Regex>>` + `pub fn new(...)`（含正则字符串构建 + 编译）。
2. `Route::borrow()` 透传 `matcher: self.matcher.clone()`。
3. `routes!` 宏 `ToTokens` 改发 `Route::new(...)`。
4. `Outlet` 改用 `r.matcher`，删除内联 `Regex::new` 与字符串构建。
5. 跑四件套（`--all-features`）+ `cargo run --example router` 验证导航与参数提取。

无需回滚策略：纯内部重构，行为等价；如出问题直接还原该变更的提交。

## Open Questions

- 是否顺带把 `Outlet` 里 `route_context.path.clone()`、`params` 的 String 分配一并优化？**倾向不**——保持本次最小、低风险，另开变更处理。
