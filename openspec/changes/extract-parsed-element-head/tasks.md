## 1. element.rs：抽出 ParsedElementHead

- [ ] 1.1 新增 `pub struct ParsedElementHead { ty: TypePath, props: Punctuated<PropsItem, Comma> }`，实现 `impl Parse`（解析 `ty` + 可选 `(props)` + rest 末位校验，**不 peek/消费 `Brace`**）
- [ ] 1.2 把 `key_span`（→ `Option<Span>`）迁移到 `impl ParsedElementHead`；`PropsItem::as_key_field` 保持不变（已是单一真源）
- [ ] 1.3 实现 `ParsedElementHead::to_element_expr(&self, children: &[ParsedElementChild]) -> TokenStream`：逐行搬移现 `ToTokens` 的 codegen（`decl_key=Uuid`、has_props/no-props 两分支、`#[allow(needless_update)]`、`set_children`），数据来源由 `self.children` 改为参数 `children`，输出 `quote!(( #block ))`（带括号表达式）

## 2. element.rs：ParsedElement 改为组合结构

- [ ] 2.1 `ParsedElement` 改为 `{ head: ParsedElementHead, children: Vec<ParsedElementChild> }`
- [ ] 2.2 `impl Parse for ParsedElement`：`let head = input.parse::<ParsedElementHead>()?;` 再 `peek(Brace)` 解析 children，组装 `{ head, children }`
- [ ] 2.3 `impl ToTokens for ParsedElement`：委托 `self.head.to_element_expr(&self.children)`
- [ ] 2.4 删除旧 `parse_head` 关联函数与旧内联 `ToTokens` codegen；确认 `ElementOrAdapter`（持有 `ParsedElement`）编译通过

## 3. router.rs：复用 head

- [ ] 3.1 `ParsedRoute.element` 字段 `ParsedElement` → `ParsedElementHead`，调整 import
- [ ] 3.2 `Parse`：`let element = input.parse::<ParsedElementHead>()?;`（不碰 `{}`）+ `element.key_span()` 有值时报错「路由组件不支持 key:」
- [ ] 3.3 `ToTokens`：对 `element.to_element_expr(&[])` 输出直接 `.into_any()`（不再手动加外层括号）；更新「括号 load-bearing」相关注释为「形状由 element 模块自洽」

## 4. 验证（行为零变化是硬约束）

- [ ] 4.1 四件套（`--all-features`）全绿：`test`（53 测试**不变**即通过，含 3 个 `routes_macro_*`）/ `clippy -D warnings` / `fmt --check` / `doc -D warnings`
- [ ] 4.2 全 examples 编译通过；逐行核对 diff 确为「仅搬移、不改逻辑」
- [ ] 4.3 `/simplify` 质量审查；更新 `dev-notes/knowledge/macros-and-props.md` 记录「head/children 类型分离 + codegen 单一真源」约定，并移除已被类型强制取代的「parse_head 不消费 {}」「括号 load-bearing」注释类提示
