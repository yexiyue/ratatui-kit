## 1. 运算符去重

- [x] 1.1 新增非门控 `reactive_ops.rs`：`macro_rules! impl_reactive_ops!($Ty)`（Add/Sub/Mul/Div + 4 个 *Assign），经 `get()`/`try_write()` 实现，全限定 `::core::ops`
- [x] 1.2 `AtomState`（原 StoreState）运算符改为 `impl_reactive_ops!(AtomState)`
- [x] 1.3 `hooks/use_state.rs`：`State` 运算符改为 `impl_reactive_ops!(State)`

## 2. Atom + use_atom

- [x] 2.1 `atom/mod.rs`：`pub struct Atom<T> { init: fn() -> T, cell: OnceLock<AtomState<T>> }` + `const fn new` + `state()` + `get/set`
- [x] 2.2 `atom/use_atom.rs`：`use_atom(&'static Atom<T>) -> AtomState<T>`，复用订阅逻辑（waker 注册/唤醒）

## 3. 删除旧机制

- [x] 3.1 删 `ratatui-kit-macros/src/store.rs` + `lib.rs` 的 `derive(Store)`/`use_stores!` + `mod store`
- [x] 3.2 清理 feature：主库 `store` → `atom`（纯主库无宏透传）；宏库 `store` feature 删除
- [x] 3.3 主库 `lib.rs`：`mod store`→`mod atom`，flatten_export 同步

## 4. 命名统一（额外）

- [x] 4.1 模块 `store/` → `atom/`；类型 `StoreState/StoreValue/StoreStateRef/Mut` → `AtomState/AtomValue/AtomStateRef/Mut`；`UseStore`/`use_store` → `UseAtom`/`use_atom`；feature `store` → `atom`

## 5. 迁移示例

- [x] 5.1 `examples/store.rs`：`#[derive(Store)]`/`XXX_STORE`/`use_stores!` → `static COUNT/VALUE: Atom<..>` + `use_atom`

## 6. 测试与验证

- [x] 6.1 `atom/mod.rs` `#[cfg(test)]`：`AtomState` 运算符/Copy + `Atom` 惰性 init/get-set/句柄共享（25 lib 单测）
- [x] 6.2 四件套全绿（`--all-features`）+ trybuild；`cargo run --example store` 编译；无 `StoreState`/`use_stores`/`feature="store"` 残留
- [x] 6.3 确认 `AtomState`/`Atom` 仍 `Send+Sync`（全局静态需 Sync，支撑 spawn 更新）
