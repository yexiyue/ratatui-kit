## 1. 确认迁移范围

- [x] 1.1 逐个核对手写 `impl Component` 的组件：`border`/`positioned` 为「字段镜像 + 收尾」模式（迁移）；`view`/`fragment` 是无字段单元结构（new 即 `Self`、update 无字段拷贝，**无重复可消**→豁免）；`modal`/`scroll_view`/`context_provider`/`textarea` 含定制逻辑→豁免
- [x] 1.2 核对候选 `new` 与 `update` 字段集一致：border（7 字段）、positioned（area+clear）均一致，确为纯镜像

## 2. 迁移规整组件

- [x] 2.1 `border`：加私有 `from_props(&BorderProps) -> Self`，`new`/`update` 改为调用之；`update` 保留 `set_layout_style` + `update_children` 显式收尾
- [x] 2.2 `positioned`：同上（`from_props` 收敛 `Rect::new(...)` + `clear`）
- [x] 2.3 `view`：无字段单元结构，无重复可消 → 归豁免（不改）
- [x] 2.4 `fragment`：无字段单元结构，无重复可消 → 归豁免（不改）

## 3. 验证

- [x] 3.1 四件套全绿（`--all-features`）：clippy `-D warnings` / fmt / test(23 lib 单测) / doc 通过
- [x] 3.2 `examples`（counter/modal/list/router 等）编译通过，行为与迁移前一致（new/update 现共用 from_props，字段集天然不会漂移）
- [x] 3.3 豁免组件（modal/scroll_view/context_provider/view/fragment）未被改动
