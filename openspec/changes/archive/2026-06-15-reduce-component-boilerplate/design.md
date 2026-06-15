## Context

`Component` trait 要求实现 `new(props) -> Self`、`update(&mut self, props, hooks, updater)`、`draw(...)`（`calc_children_areas`/`poll_change` 有默认）。规整组件（border/positioned/view/fragment）的典型实现：

```rust
fn new(props) -> Self { Self { a: props.a, b: props.b, ... } }      // 字段列表第 1 遍
fn update(&mut self, props, _, updater) {
    *self = Self { a: props.a, b: props.b, ... };                    // 字段列表第 2 遍
    updater.set_layout_style(props.layout_style());
    updater.update_children(&mut props.children, None);
}
fn draw(...) { /* 定制 */ }
```

字段列表写两遍是漂移源。`draw` 始终定制。

## Goals / Non-Goals

**Goals:**
- 规整组件的镜像字段列表只写一处，`new`/`update` 共享之。
- 迁移低风险、行为等价、可逐个组件进行。

**Non-Goals:**
- 不试图自动生成 `draw` 或 `calc_children_areas`（无法/不应派生）。
- 不强迁有定制逻辑的组件（modal/scroll_view/context_provider）。
- 不改 `Component` trait 签名、不改公开 API/DSL。

## Decisions

### 决策 1：用 `from_props` 私有构造源约定，而非 `#[derive(Component)]`

每个规整组件加私有 `fn from_props(props: &Self::Props<'_>) -> Self`，`new`/`update` 都调用它：

```rust
impl Border {
    fn from_props(props: &BorderProps) -> Self { Self { a: props.a, b: props.b, ... } }
}
impl Component for Border {
    fn new(props) -> Self { Self::from_props(props) }
    fn update(&mut self, props, _, updater) {
        *self = Self::from_props(props);
        updater.set_layout_style(props.layout_style());
        updater.update_children(&mut props.children, None);
    }
    fn draw(...) { /* 不变 */ }
}
```

**为什么不用 derive**：`Component` 的 `draw` 必填且定制，而一个 trait impl 块不可拆分——`#[derive(Component)]` 若生成 `impl Component for X { new, update }`，用户就无处再补 `draw`（不能有第二个 `impl Component for X`）。要 derive 只能生成「另一个 trait」（如 `from_props` 载体 trait），用户仍要写 `new`/`update` 包装去调它——并未真正省掉包装样板，反增一层间接与一个新宏的维护面。`from_props` 约定零新宏、零间接、风险最低，且已消除真正的重复源（字段列表）。

**备选（derive 宏）已否决**：成本（新宏 + 跨 crate 维护 + 仍需手写包装）高于收益。若未来希望「强制」而非「约定」，可再开变更引入。

### 决策 2：`from_props` 只构造自身状态，updater 收尾留在 `update`

`set_layout_style` / `update_children` 操作的是 updater 而非自身字段，且 `new` 不做这些（`new` 只构造）。故它们留在 `update` 显式调用，`from_props` 保持纯粹（仅 `&props -> Self`，便于 `new` 复用）。

### 决策 3：迁移范围按「是否纯字段镜像」判定

- **迁移**：`new` 为纯 `Self { 字段 <- props }`、`update` 为「重建 + set_layout_style + update_children」的组件——目测 `border`、`positioned`、`view`、`fragment`（逐个确认）。
- **豁免**：`modal`（条件/弹层逻辑）、`scroll_view`（自管 ScrollViewState）、`context_provider`（特殊 value 处理）、`textarea`（已下线）。豁免组件不动。

## Risks / Trade-offs

- **[约定非强制]** → 新组件作者可能仍写重复 `new`/`update`。本次以「示例 + 既有组件树立范式」缓解；强制化留待未来 derive 变更（若确有需要）。
- **[逐字迁移引入笔误]** → 行为须与原实现等价；每迁一个组件即跑 `examples` 冒烟（counter/modal/list/router）。
- **[个别组件 `new`/`update` 已有细微差异]** → 迁移前逐个核对 `new` 与 `update` 的字段集是否真一致；若不一致说明本就是 bug 或属定制，归入豁免或单独修正。

## Migration Plan

1. 逐个组件确认属「纯字段镜像」模式。
2. 为其加私有 `from_props`，改写 `new`/`update` 调用之。
3. 每改一个跑四件套（`--all-features`）+ 相关 example。
4. 豁免组件不动。

回滚：纯重构，逐组件提交，出问题还原对应提交即可。

## Open Questions

- `view`/`fragment` 的 `new`/`update` 是否足够规整可纳入？——实现时逐个核对，不规整则归豁免，不强行。
