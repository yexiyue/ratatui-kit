## Context

现有 store：`#[derive(Store)]` 把 struct 每字段炸成 `StoreState<T>`，生成 `XXX_STORE` 全局单例，`use_stores!` 订阅字段。`StoreState<T>` 已是「全局 Owner + 多订阅 waker(`HashMap<ElementKey,Waker>`)」的响应式句柄,与 `State<T>` 运算符 ~99% 重复。TRNovel 不用它。改为 Atom 式：保留 `StoreState` 作句柄,换掉笨重的声明/订阅机制。

## Goals / Non-Goals

**Goals:**
- 零宏/零结构地声明全局原子;任何地方直接读写(含 spawn);细粒度订阅;复用 State 核心、消除重复。

**Non-Goals:**
- 不统一 `State` 与 `StoreState` 为同一类型(订阅模型不同:本地单订阅 vs 全局多订阅)——只共享运算符实现。
- 不引入 selector/派生原子(未来可加);本次只做基础 atom。
- 不动 `State`(本地状态)的对外行为。

## Decisions

### 决策 1：`Atom<T>` = `const fn` + `OnceLock` 惰性

```rust
pub struct Atom<T: Send + Sync + 'static> {
    init: fn() -> T,
    cell: OnceLock<StoreState<T>>,
}
impl<T: Send + Sync + 'static> Atom<T> {
    pub const fn new(init: fn() -> T) -> Self { Self { init, cell: OnceLock::new() } }
    fn state(&self) -> StoreState<T> { *self.cell.get_or_init(|| StoreState::new((self.init)())) }
    pub fn get(&self) -> T where T: Copy { self.state().get() }
    pub fn set(&self, v: T) { self.state().set(v) }
}
```

- `fn() -> T`(无捕获)→ const 友好,可作 `static`。generational-box 运行时建,故 `OnceLock` 惰性(像 `LazyLock`)。
- `Atom<T>: Sync`(`OnceLock<StoreState>` + `fn` 均 Sync)→ 可作 `static`。

### 决策 2：复用 `StoreState<T>` 作句柄,只换声明/订阅机制

`StoreState`/全局 `OWNER`/多订阅 waker 机制**保留**。`use_store(StoreState)` → `use_atom(&'static Atom<T>)`：内部 `atom.state()` 惰性解析 + 复用 `UseStoreImpl` 的「poll_change 注册 waker、写入唤醒全部」逻辑。返回 `StoreState`(Copy+Send,可移进 spawn)。

### 决策 3：`impl_reactive_ops!` 宏消除运算符重复

把 `Add/Sub/Mul/Div` + `*Assign`(共 8 类)抽成 `macro_rules! impl_reactive_ops!($Ty)`,对 `State` 与 `StoreState` 各调一次。两者都有 `read()`/`write()`/`get()`/`set()`,宏体经这些方法实现,不碰各自内部存储差异。

### 决策 4：删除旧机制

删 `ratatui-kit-macros/src/store.rs` + `lib.rs` 的 `derive(Store)`/`use_stores!` proc-macro 入口;主库 `store` feature 不再透传 `ratatui-kit-macros/store`(宏库 `store` feature 可一并删)。主库 `lib.rs` 的 store 导出去掉 `use_stores!`/`Store`,加 `Atom`/`use_atom`。

## Risks / Trade-offs

- **[纯 atom 无 struct 内聚]** → 相关状态是多个独立 static(COUNT/NAME/...)。这是 Jotai 式取舍,可组合、清晰;用户已选此方向。
- **[运算符宏化]** → 宏体须同时适配 State/StoreState 的方法签名;先确认两者 `read/write/get/set` 一致再抽,trybuild/编译兜底。
- **[破坏性]** → 旧 `derive(Store)`/`use_stores!` 用户代码失效;不顾兼容,CHANGELOG 注明,examples 同步改。

## Migration Plan

1. `store/mod.rs`：抽 `impl_reactive_ops!`,改 `StoreState` 运算符为宏;加 `Atom<T>` + `get/set`。
2. `hooks/use_state.rs`：`State` 运算符改用 `impl_reactive_ops!`。
3. `store/use_store.rs`：`use_store` → `use_atom(&'static Atom<T>)`,复用订阅逻辑。
4. 删宏库 `store.rs` + `lib.rs` proc-macro 入口;清 feature 透传。
5. 主库 `lib.rs`：store 导出更新(去 use_stores!/Store,加 Atom/use_atom)。
6. `examples/store.rs` 改写为 atom。
7. 四件套(`--all-features`)+ 现有测试全绿;给 atom 补单测(惰性 init、set/get、细粒度——可在 store/mod.rs `#[cfg(test)]`)。

回滚：逐步提交,出问题还原。

## Open Questions

- `use_atom` 是否也提供 `use_atom_value`(只读、不订阅写)?——本次先做读写订阅版,按需再加。
