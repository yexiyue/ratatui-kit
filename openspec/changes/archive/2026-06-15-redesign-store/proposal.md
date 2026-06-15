## Why

现有全局 store「用处不大」有实锤：主力真实应用 **TRNovel 完全不用它**（0 处），跨组件状态全靠 `use_state`(根) + `ContextProvider` + `use_context`（41 处）。设计缺陷：每个 struct 一个全局单例（不可多实例/scope/测试隔离）、每字段炸成独立 `StoreState`、`StoreState` 与 `State` ~99% 重复（25 块运算符重抄）、`CounterAndTextInput → COUNTER_AND_TEXT_INPUT_STORE` 命名 hack + `use_stores!`/`#[derive(Store)]` 重宏。

改为 **Atom 式全局原子**（类 Jotai/Recoil）：零宏、零结构、细粒度、复用 State 核心。store 相对 context+state 的独有价值就此清晰——**真·全局、零样板的响应式状态，任何地方直接读写，无需上提/Context**。

## What Changes

- **新增 `Atom<T>`**：模块级 `static COUNT: Atom<i32> = Atom::new(|| 0);`。`const fn new(init: fn() -> T)`，内部 `OnceLock<StoreState<T>>` 惰性建（首次 `use_atom` 时 insert 进全局 OWNER）。提供 `Atom::get/set` 供组件外/spawn 直接读写。
- **新增 `use_atom(&'static Atom<T>) -> StoreState<T>`**：解析 atom → 注册本组件 waker → 返回 `Copy + Send` 句柄（可移进 `tokio::spawn`）。复用现有多订阅 waker 机制。
- **消除 `State`/`StoreState` 运算符重复**：抽 `macro_rules! impl_reactive_ops!`，给两者各生成一次（Add/Sub/Mul/Div + Assign）。
- **删除** `#[derive(Store)]`、`use_stores!`、宏库 `Store`/`UseStores`、`XXX_STORE` 生成机制及主库/宏库的 `store` feature 透传。
- **改写** `examples/store.rs` 为 atom 写法。

## Capabilities

### New Capabilities
- `global-atoms`: 全局响应式原子契约——`Atom<T>` 惰性声明、`use_atom` 订阅、细粒度按 atom 重渲、组件外/后台 spawn 直接读写、复用 `StoreState` 的 `Send+Sync` 全局句柄。

### Modified Capabilities
<!-- 无:openspec/specs/ 当前为空。 -->

## Impact

- **代码**：`store/mod.rs`（加 `Atom`、`impl_reactive_ops!`，保留 `StoreState`/`OWNER`/waker 机制）、`store/use_store.rs`（`use_store` → `use_atom`）、`hooks/use_state.rs`（运算符改用宏）、`lib.rs`（`store` feature 导出更新）。
- **删除**：`ratatui-kit-macros/src/store.rs` + 其 `derive(Store)`/`use_stores!` proc-macro 入口；主库/宏库 `store` feature 的 `ratatui-kit-macros/store` 透传。
- **公开 API**：`#[derive(Store)]`/`use_stores!`/`XXX_STORE` 移除，改 `Atom`/`use_atom`——**破坏性**（不顾兼容）。
- **Send**：`StoreState`/`Atom` 仍 `Send + Sync`（全局静态需 Sync）——drop-send-sync 后 Send 合理保留的唯一处，正好支撑后台 spawn 更新。
- **风险**：中——重写 store + 宏删除 + 运算符宏化；examples 回归 + 四件套验证。
