## Why

`routes!` 传 props 的实现（commit `7a6a912`）让 `ParsedElement` 既管解析又管 codegen，留下两个**跨文件隐性契约**，目前仅靠「doc 注释 + 测试/编译」双层护栏维持：(A) `ParsedElement::parse_head` 绝不能消费 `{}`，且它造出 `children: Vec::new()` 的半成品占位；(B) `ParsedElement::ToTokens` 发出 `{...}` block，故 `router.rs` 的 `(#element).into_any()` 靠外层括号 load-bearing。功能完全正确，但「head（ty+props）」与「children」两个关注点没在类型上分离——而 macros（`element!`/codegen）近期还会继续演进（hook 优化、textarea 重写、docs 国际化均可能再碰此处），把契约从注释提升为类型/机制强制可持续回本。

## What Changes

关键杠杆：**`children` 从「字段」降为「codegen 参数」**。

- 新增 `ParsedElementHead { ty, props }`（**无 `children` 字段**）——承载「ty + 可选 `(props)`」的解析与 element codegen，并持有 `key_span` / `as_key_field`。其 `parse` 结构上不可能触及 `{}`，契约 A 消失。
- `ParsedElementHead` 拥有 codegen：`fn to_element_expr(&self, children: &[ParsedElementChild]) -> TokenStream`，输出**带括号的 `Element<Ty>` 构造表达式** `({ … _element })`，调用方可直接 `.method()`，契约 B 消失。
- `ParsedElement` 重构为 `{ head: ParsedElementHead, children: Vec<ParsedElementChild> }`；其 `ToTokens` 委托 `self.head.to_element_expr(&self.children)`；`Parse` 先 `ParsedElementHead::parse` 再 `peek(Brace)` 填 `children`。
- `router.rs`：`ParsedRoute.element` 改为 `ParsedElementHead`；`Parse` 用 `ParsedElementHead::parse` + `key_span` 拒绝 `key:`；`ToTokens` 用 head 的带括号表达式直接 `.into_any()`，**不再手动加括号、不再依赖 token 形状**。
- 副效果：`routes!` 持有的是 head，**物理上没有 `children` 可传** →「路由组件无静态 children」从语义约定变为结构强制；`children: Vec::new()` 占位 smell 与两条 load-bearing 注释一并消失。

非 BREAKING：纯内部重构，`element!` / `routes!` 的对外语法与展开行为**零变化**。

## Capabilities

### New Capabilities

- `element-head-codegen`: 确立「element 头部（ty+props）的解析与 codegen 作为独立单元、`children` 作为 codegen 参数注入」这一结构性约定——含 head 无 children 字段、codegen 单一真源、`routes!` 复用 head 且无法传 children、codegen 输出形态自洽（调用方不依赖 token 形状）等不变量。

### Modified Capabilities

<!-- 无：纯重构，不改任何对外 spec 行为。router-path-matching 等现有 capability 的 requirements 不变。 -->

## Impact

- **代码**：`packages/ratatui-kit-macros/src/element.rs`（拆出 `ParsedElementHead`、`ParsedElement` 结构改造、`ToTokens` 重写、`key_span`/`as_key_field` 归属迁移到 head）、`packages/ratatui-kit-macros/src/router.rs`（`element` 字段类型 + Parse + ToTokens）。
- **波及面**：`ElementOrAdapter` 持有 `ParsedElement`，需确认 `ToTokens` 委托后不受影响；`adapter.rs` 不涉及。
- **风险**：`element!` 是全框架最热宏，所有组件过其 codegen；项目「无单元测试」，仅靠 53 个测试 + 全 examples 编译兜底。
- **验证标准**：行为零变化——四件套（`--all-features`）全绿 + 现有 `routes_macro_*` 与全部 53 测试**不变即通过** + examples 全编译。
