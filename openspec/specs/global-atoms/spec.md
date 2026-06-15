# global-atoms Specification

## Purpose
TBD - created by archiving change redesign-store. Update Purpose after archive.
## Requirements
### Requirement: Atom 惰性声明为模块级静态

`Atom<T>` SHALL 可作模块级 `static` 声明（`Atom::new` 为 `const fn`，接受无捕获的 `fn() -> T` 初始化器）。其底层响应式状态 SHALL 在首次被 `use_atom`/读写时**惰性**创建（插入进程级全局 Owner），而非声明即建。

#### Scenario: 静态声明且惰性初始化
- **WHEN** 声明 `static COUNT: Atom<i32> = Atom::new(|| 0)` 且程序从未访问它
- **THEN** 编译通过且不创建任何底层状态；首次 `use_atom(&COUNT)`/`COUNT.get()` 时才以 `0` 初始化

### Requirement: use_atom 订阅并返回可跨线程句柄

`use_atom(&'static Atom<T>)` SHALL 注册本组件的 waker 到该 atom，并返回一个 `Copy + Send` 的响应式句柄（复用 `StoreState<T>`）。写入该句柄 SHALL 仅唤醒订阅了**该 atom** 的组件（细粒度）。

#### Scenario: 细粒度重渲
- **WHEN** 组件 A 订阅 `COUNT`、组件 B 订阅 `NAME`，随后写 `COUNT`
- **THEN** 仅 A 重渲，B 不重渲

#### Scenario: 句柄可移入 tokio::spawn
- **WHEN** `let c = hooks.use_atom(&COUNT); tokio::spawn(async move { c.set(load().await) })`
- **THEN** 后台任务写入后，订阅 `COUNT` 的组件被唤醒重渲

### Requirement: 组件外直接读写

`Atom<T>` SHALL 提供 `get`(T: Copy)/`set` 以在非组件代码（含后台任务持 `&'static Atom`）中直接读写，语义与经句柄读写一致。

#### Scenario: 组件外写入触发订阅者重渲
- **WHEN** 非组件代码调用 `COUNT.set(5)` 且有组件订阅了 `COUNT`
- **THEN** 该组件被唤醒并读到 `5`

### Requirement: 移除旧 store 机制

`#[derive(Store)]`、`use_stores!`、`XXX_STORE` 静态生成 SHALL 移除。全局状态 MUST 只经 `Atom`/`use_atom` 表达。

#### Scenario: 旧 API 不复存在
- **WHEN** 检索 `derive(Store)`/`use_stores!`/`StoreState::new` 的对外用法
- **THEN** 框架不再导出 `#[derive(Store)]`/`use_stores!`；全局状态示例改用 `Atom`

### Requirement: State 与 StoreState 运算符不重复

`State<T>` 与 `StoreState<T>` 的算术运算符重载（`Add/Sub/Mul/Div` 及 `*Assign`）SHALL 由单一 `macro_rules!` 生成，二者 MUST NOT 各自手抄一遍。

#### Scenario: 运算符单一来源
- **WHEN** 审视运算符 impl
- **THEN** `State` 与 `StoreState` 经同一宏展开，无重复代码块

