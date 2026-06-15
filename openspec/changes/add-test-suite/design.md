## Context

全库仅 `history.rs` 有 2 个单测。三层各有不同的可测性：

- **宏**：纯 token 变换，trybuild 编译测试最合适；但展开生成 `::ratatui_kit::...`，须在运行时 crate 内编译。
- **运行时逻辑**：`ElementKey`/`multimap`/`State` 运算符/`history` 是纯逻辑，`#[cfg(test)]` 单测直接可测；协调复用、hooks 顺序依赖组件树驱动，较难纯单测。
- **组件渲染**：`render()`（`render/tree.rs:34`）是**同步**的「update + draw」，异步只在事件等待循环；故可做「单次离屏渲染」，但需访问内部（`Tree`/`render()`/`ComponentDrawer`）。

## Goals / Non-Goals

**Goals:**
- 三层都有可执行回归网，随 `cargo test --tests` 跑，不改 CI。
- 失败用例稳定（不被 rustc 版本拖累）。
- 提供可复用的「渲染组件到 Buffer」测试 harness。

**Non-Goals:**
- 不追求 100% 覆盖；优先关键路径与易回归点。
- 不测试真实终端 I/O、不测试异步事件循环本身。
- 不为难以纯测的部分（如完整协调流程）强造脆弱测试——能经渲染 harness 间接覆盖即可。

## Decisions

### 决策 1：三层分治

| 层 | 手段 | 位置 |
|---|---|---|
| 宏 | trybuild pass/fail | `ratatui-kit/tests/ui/` |
| 运行时逻辑 | `#[cfg(test)] mod tests` 就近 | 各 `src/**` 模块内 |
| 组件渲染 | TestBackend Buffer 断言 | `ratatui-kit/tests/` 或 `#[cfg(test)]` |

### 决策 2：宏 trybuild —— 位置与稳定 stderr

沿用窄变更的两条结论：UI 测试放 `ratatui-kit` crate（展开路径可解析）；失败用例只断言本库 `syn::Error` 文案（旧 `$`/`#()`、`widget`/`stateful` 参数错误、`#[component]` 非法参数名等），不绑定 rustc 类型错误。`routes!`/`use_stores!`/`#[derive(Store)]` 用例需在对应 feature 下（CI 已 `--all-features`）。

### 决策 3：组件渲染 harness —— 单次同步渲染到 Buffer

提供 test-only 入口：把 `element` 转 `AnyElement` → 建 `Tree`（`Tree::new(props, helper)`）→ 调一次同步 `render()` 把树绘到指定尺寸的 `Buffer` → 返回 `Buffer` 供断言。

```rust
// 形如
fn render_to_buffer(el: impl Into<AnyElement<'static>>, w: u16, h: u16) -> ratatui::buffer::Buffer
```

- **plumbing 待定**：`render()` 与 `Tree` 当前是 crate 私有。两条落地路径，apply 时择一：
  - (A) 在 crate 内加 `#[cfg(test)]`/`pub(crate)` 的一次性渲染 helper，复用现有 `render()` + 一个基于 `TestBackend` 的 `Terminal` 后端；
  - (B) 直接构造 `ComponentDrawer`（持 `&mut Buffer` + area）调树的 `draw`，绕过 `Terminal`——更轻量但需暴露少量内部。
  倾向 (B)（不引入 TestBackend 终端封装），具体看 `ComponentDrawer` 构造可达性。
- **静态组件优先**：harness 不轮询 future，仅测「一次渲染的静态输出」；带 `use_future`/事件的动态行为不在本 harness 范围。

### 决策 4：运行时单测范围按「可纯测」划线

- **直接单测**：`ElementKey`（Decl/User 不碰撞、Hash/Eq）、`multimap`、`State`/`StoreState` 运算符与 `Copy`、`history` 越界、`router` 路径匹配函数。
- **经渲染 harness 间接覆盖**：组件 `new`/`update`/`draw` 的产出。
- **暂不强测**：完整协调（reconciliation）跨帧状态保持——成本高、收益靠渲染 harness 部分覆盖即可，留作后续。

### 决策 5：与其它变更的顺序

- `router` 路径匹配单测在 [`cache-router-regex`] 把匹配抽成 `Route` 上的可测函数后**显著更好写**。建议 **apply 顺序：先 `cache-router-regex` → 再本变更的 router 部分**；或先用渲染 harness 间接测路由。
- 与 [`reduce-component-boilerplate`] 正交：组件渲染测试正好是该重构的回归网，**建议本变更（至少渲染 harness + 组件渲染测试）先于组件重构落地**，给重构兜底。

## Risks / Trade-offs

- **[.stderr 版本漂移]** → 只断言本库文案；残余用 `TRYBUILD=overwrite` 重生成并复核。
- **[渲染 harness 耦合内部]** → 需暴露少量 `pub(crate)`/`#[cfg(test)]` 入口；控制暴露面，仅供测试。
- **[异步/future 组件难测]** → harness 明确只覆盖静态单次渲染；动态行为不纳入，避免脆弱异步测试。
- **[偏离「无单元测试」约定]** → 本变更即在反转该约定，并同步更新 `toolchain.md`。

## Migration Plan

1. 加 `trybuild` dev-dep + `tests/ui.rs` harness；按宏逐个补 pass 用例。
2. 补 fail 用例 + `TRYBUILD=overwrite` 生成并复核 `.stderr`。
3. 就近补运行时逻辑单测（ElementKey/multimap/State/history/router-matching）。
4. 落地渲染 harness（择 A/B），补 Border/Text/View/Center 渲染测试。
5. 更新 `toolchain.md` 测试约定。
6. 四件套（`--all-features`）全绿。

回滚：纯新增；删除 `tests/`、各 `#[cfg(test)]`、dev-dep 与 harness 入口即可。

## Open Questions

- 渲染 harness 走 (A) Terminal<TestBackend> 还是 (B) 直接 ComponentDrawer over Buffer？——apply 时看 `ComponentDrawer`/`Tree` 的最小暴露面定夺，倾向 (B)。
- 协调跨帧状态保持是否值得专门测？——本次先靠渲染 harness 间接覆盖，按需另开。
