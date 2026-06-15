# element-head-codegen Specification

## Purpose

`element!` 与 `routes!` 共享的「元素头部（ty + props）解析 + element codegen」机制。头部由不含 children 的 `ParsedElementHead` 承载、children 作为 codegen 参数注入，使「无 children」「codegen 单一真源」「调用方不依赖 token 形状」成为类型/结构层面的保证而非注释约定。由变更 `extract-parsed-element-head` 引入。

## Requirements

### Requirement: head 与 children 类型分离

element 的「头部」（类型路径 + 可选 `(props)`）解析 SHALL 由独立的 `ParsedElementHead` 承载，该类型 MUST NOT 含 `children` 字段——使「头部解析阶段触及子节点块 `{}`」在类型层面无法表达，无需依赖注释约定。

#### Scenario: 头部解析结构上不可能消费子节点块

- **WHEN** 任意调用方解析一个 element 头部（`element!` 的 `Parse` 或 `routes!` 的 `ParsedRoute::parse`）
- **THEN** 它得到的是仅含 ty + props 的 `ParsedElementHead`，`{}` 仍留在输入流中由调用方自行决定归属（`element!` 当 children、`routes!` 当子路由），头部类型本身没有承接 children 的字段

### Requirement: element codegen 单一真源

`Element<Ty>` 的构造 codegen SHALL 仅由 `ParsedElementHead` 提供（单一真源）；`children` MUST 作为 codegen 的参数注入，而非从持有状态读取。`element!` 与 `routes!` MUST 经由同一 codegen 路径生成元素。

#### Scenario: element! 与 routes! 共享同一 codegen

- **WHEN** `element!(Comp(prop: v))` 与 `routes!{ "/x" => Comp(prop: v) }` 各自展开
- **THEN** 两者生成的 `Element<Comp>` 构造代码经由同一个 `ParsedElementHead` codegen 入口产出，no-props 路径与改造前字节等价，`key:` / `..rest` / `(expr).into()` 行为一致

#### Scenario: children 作为参数而非字段

- **WHEN** codegen 需要为带子节点的 element 注入 children
- **THEN** children 以参数形式传入 codegen（`element!` 传实际子节点切片、`routes!` 传空切片），codegen 不读取任何「持有的 children 状态」

### Requirement: routes! 无法传入静态 children

`routes!` 的路由元素 SHALL 持有 `ParsedElementHead`，使路由组件在结构上 MUST NOT 能携带静态 children——「路由组件无静态 children，子节点位置 `{}` 归嵌套子路由」从语义约定升级为类型强制。

#### Scenario: 路由组件无静态 children 是结构强制

- **WHEN** 编写 `routes!{ "/a" => Comp(prop: v) { "/b" => Sub } }`
- **THEN** `Comp` 后的 `{}` 被解析为嵌套子路由而非 `Comp` 的静态子节点；路由元素持有的 head 类型根本没有承接静态 children 的能力

### Requirement: codegen 输出形态自洽

`ParsedElementHead` 的 codegen 输出 SHALL 是可直接参与方法调用的表达式形态（自带必要的包裹）；调用方 MUST NOT 依赖其内部 token 结构（是否为 block、是否需外层括号）。

#### Scenario: router 内嵌元素无需手动加括号

- **WHEN** `routes!` 的 `ToTokens` 需要把元素转成 `AnyElement` 并传入 `Route::new`
- **THEN** 它直接对 codegen 输出调用 `.into_any()`，无需自己补外层括号、无需知道输出内部是块表达式

### Requirement: 重构保持行为零变化

本能力的引入 SHALL 是纯内部重构，`element!` / `routes!` 的对外语法与展开行为 MUST 保持不变；全部既有测试与 examples MUST 不经修改即通过。

#### Scenario: 现有测试与 examples 不变即通过

- **WHEN** 应用本重构后运行四件套（`--all-features`）
- **THEN** 全部既有测试（含 `routes_macro_accepts_props` / `routes_macro_accepts_props_with_children` / `routes_macro_no_props_still_works` 等 53 个测试）与全部 examples 编译，均不经修改即通过
