## 1. Route 缓存载体

- [x] 1.1 在 `Route` 新增私有字段 `matcher: Option<Arc<regex::Regex>>`（`components/router/mod.rs`）
- [x] 1.2 实现 `pub fn Route::new(path, component, children) -> Route`：把现 `Outlet` 内的「按 `/` 切分 + `:name` → `(?<name>[^/]+)` + join」正则字符串构建逻辑迁入；含 `/:` 时编译为 `Some(Arc<Regex>)`（非法正则在此 `expect`/panic，构建期暴露），否则 `None`
- [x] 1.3 `Route::borrow()` 透传 `matcher: self.matcher.clone()`（Arc 共享，跨帧复用）；另加 `pub(crate) fn matcher()` 访问器

## 2. routes! 宏改构造方式

- [x] 2.1 `ToTokens for ParsedRoute` 改为生成 `Route::new(#path.to_string(), <component>, <children>.into())` 而非 `Route { .. }` 结构体字面量（`ratatui-kit-macros/src/router.rs`）

## 3. Outlet 改用缓存正则

- [x] 3.1 删除 `Outlet` 中内联的正则字符串构建与 `regex::Regex::new(...).expect(...)`（`components/router/outlet.rs`）
- [x] 3.2 改用 `r.matcher()`：`Some(re)` 走动态匹配（`find`/`captures` 提取参数），`None` 继续静态段边界匹配，语义保持不变

## 4. 验证

- [x] 4.1 四件套全绿（`--all-features`）：`cargo test`/`clippy -D warnings`/`fmt --check`/`RUSTDOCFLAGS="-D warnings" cargo doc`
- [x] 4.2 `cargo run --example router` 编译通过（TUI 需交互终端，本环境以编译 + 语义逐字保留为验证；正则 pattern 与匹配逻辑未变，仅改为复用缓存）
- [x] 4.3 确认非动态路由 `matcher = None` 不编译正则；非法路由路径在 `Route::new`（构造期）`expect` 报错，而非每次渲染
