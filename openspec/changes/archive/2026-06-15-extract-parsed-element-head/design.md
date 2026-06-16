## Context

`routes!` 传 props（commit `7a6a912`）后，`ParsedElement` 同时承担解析 `ty (props) {children}`、持有 `children` 状态、生成 `Element<Ty>` codegen 三件事。为让 `routes!` 复用 props 解析，抽出了 `parse_head`（只解析头部、不消费 `{}`，留 `children: Vec::new()` 占位），并让 `router.rs` 内嵌 `(#element).into_any()`。由此产生两个跨文件隐性契约（详见 proposal 的契约 A/B），靠注释 + 测试/编译双层护栏维持。

约束：
- `element!` 是全框架最热宏，所有组件（含 examples）过其 `ToTokens`。
- 项目「无单元测试」，回归网 = 53 个 macros/router 测试 + 全 examples 编译。
- 必须**行为零变化**（纯重构）。

## Goals / Non-Goals

**Goals:**
- 让「parse 阶段不碰 children」在类型层面不可违反（head 无 `children` 字段）。
- 让「element codegen」有单一真源，`router.rs` 不依赖其 token 形状。
- 消除 `children: Vec::new()` 占位与两条 load-bearing 注释。

**Non-Goals:**
- 不改 `element!` / `routes!` 的对外语法或展开行为。
- 不动 `#[component]` / `#[with_layout_style]` / `adapter` codegen。
- 不引入新依赖、不改性能特征。

## Decisions

### D1：`children` 从字段降为 codegen 参数（核心杠杆）

```
当前：ParsedElement 持 children 字段，ToTokens 读 self.children
┌──────────────────────────────────────────┐
│ ParsedElement { ty, props, children }     │
│   parse_head() → children: Vec::new()     │ ← 占位 smell
│   ToTokens: 读 self.children 注入          │
└──────────────────────────────────────────┘
      ▲ router 复用 parse_head 拿到 children=[] 的半成品（契约 A 隐性）

重构后：head 无 children，codegen 把 children 当参数
┌──────────────────────────┐      ┌───────────────────────────────────┐
│ ParsedElementHead        │      │ ParsedElement { head, children }  │
│   { ty, props }          │◀─────│   ToTokens:                        │
│   impl Parse  ← 不碰 {}  │ 组合 │     head.to_element_expr(&children) │
│   to_element_expr(children)     └───────────────────────────────────┘
│   key_span / as_key_field │
└──────────────────────────┘
      ▲ router 持 head，物理上无 children 可传 → 传 &[]（契约 A 结构消除）
```

**rationale**：把「谁拥有 element codegen」收敛到 head，`children` 作为输入参数注入。则 `head` 的解析结构上无 children 来源（契约 A 由类型消除），`router` 持 head 也无 children 来源（"路由无静态 children"从语义约定变结构强制）。

**备选**：①「保留字段 + 仅加注释」收益小、契约仍隐性；②「保留 ParsedElement 单类型、只抽 `to_element_expr` 方法」（探索中的方案 B）解决不了契约 A，且 `parse_head` 仍可能被误改为消费 `{}`。

### D2：`to_element_expr` 输出「带括号表达式」而非裸 block

`element!` codegen 本质是多语句块 `{ type Props=…; let _props=…; let _element=…; <children注入> _element }`，必须是 block。痛点：block 在实参位 `{...}.into_any()` 会被解析成「语句块 + 游离表达式」，故现状靠 `router` 手动加括号。

**决策**：`to_element_expr` 返回 `quote!(( #block ))`——自带外层括号。则：
- `element!` 主路径：`ParsedElement::to_tokens` 直接 extend 该表达式（`({...})` 作为求值表达式合法）。
- `router`：`(#head_element).into_any()` 中 `#head_element` 已是 `({...})`，`.into_any()` 合法，router **不再需要知道里面是 block**——token 形状知识收归 element 模块。

**备选**：裸 block + 调用方加括号（现状，契约 B 隐性）；闭包包裹 `(|| { … })()`（多一层调用、更丑）。带括号表达式最轻。

### D3：`ParsedElementHead` 实现 `Parse`；`key_span` / `as_key_field` 归属 head

- head 实现 `impl Parse`（解析 `ty` + 可选 `(props)` + rest 末位校验）。`ParsedElement::parse` = `input.parse::<ParsedElementHead>()?` 再 `peek(Brace)` 填 children；`router` 的 `ParsedRoute::parse` = `input.parse::<ParsedElementHead>()?`（不碰 `{}`，`{}` 归子路由）。`parse_head` 关联函数被 head 的 `Parse` 取代。
- `key_span` / `as_key_field` 只读 `props`，随 props 迁入 head。`router` 经 `head.key_span()` 拒绝 `key:`，语义不变。

### D4：has_props / no-props 两分支整体迁入 `to_element_expr`

现 `ToTokens` 的两个 codegen 分支（has_props_assignments 带 `..Default::default()` + `#[allow(needless_update)]`；无则 `Props::default()`）、`set_children` 注入、`decl_key = Uuid::new_v4()` 全部随 codegen 迁入 `to_element_expr`。逻辑**逐行搬移、不改**，仅把数据来源从 `self.children` 换成参数 `children`（空 → 无 `set_children`）。

## Risks / Trade-offs

- [element! 热路径回归] → 行为零变化是硬约束。缓解：四件套（`--all-features`）全绿 + 全部 53 测试不变 + examples 全编译；diff 应可逐行对照「仅搬移、不改逻辑」。
- [lifetime / 借用] → `to_element_expr(&self, children: &[ParsedElementChild]) -> TokenStream` 返回 owned token，children 仅生成期被借用，无生命周期纠葛；`ParsedElement::to_tokens` 传 `&self.children`，router 传 `&[]`。
- [ElementOrAdapter 波及] → 它仅依赖 `ParsedElement` 的 `ToTokens`；委托后接口不变，编译通过即安全。
- [decl_key 时机] → `Uuid::new_v4()` 调用点随迁移保持「每次 `to_element_expr` 一次」，与现状「每个 element 一个 decl_key」一致。

## Migration Plan

纯编译期重构，无运行时迁移、无数据迁移。回滚 = 还原 `element.rs` / `router.rs` 两文件。落地步骤见 tasks.md。

## Open Questions

- `to_element_expr` 的可见性：倾向 `ParsedElementHead` 私有方法（仅 `ParsedElement::to_tokens` 与 `router` 经同模块/`crate::element` 调用）。若 router 跨模块调用需 `pub(crate)`——实现期按可见性报错定，优先最小可见性。
