## Why

全库目前**基本无测试**——仅 `router/history.rs` 有 2 个单测，其余靠「examples/docs 能编译」间接把关。而近期对宏（`element!` 去 sigil、控制流）与运行时（`ElementKey` 枚举化、adapter 按引用）做了大改，缺乏真正的回归网。需要跨「宏 / 运行时逻辑 / 组件渲染」三层补齐测试，让后续重构有保障。

本变更取代并扩大原 `macro-trybuild-tests` 提案（只覆盖 `element!`），改为**全库覆盖**。

## What Changes

- **宏编译测试（trybuild）覆盖全部宏**：`element!`、`#[component]`、`#[derive(Props)]`、`routes!`、`use_stores!`、`#[derive(Store)]`、`#[with_layout_style]`——通过用例验证正常展开，失败用例断言本库稳定报错（旧 `$`/`#()` 迁移提示、参数错误、参数名错误等）。
- **运行时逻辑单测**：`ElementKey`（Decl/User 不碰撞、Hash/Eq）、`multimap`、协调复用语义（同 key+type 复用、异则新建）、`use_state`/`store`（运算符重载、`Copy`、写入触发 waker）、`router` 路径匹配（段边界、参数提取）与 `history` 边界（go/forward 越界）、布局 `calc_children_areas`。
- **组件渲染测试**：引入「单次离屏渲染到 `ratatui::backend::TestBackend` 的 Buffer」测试 harness，对代表性组件（`Border`/`Text`/`View`/`Center`/`Modal`/`ScrollView`）渲染并断言 Buffer 内容。
- **更新约定**：`toolchain.md` 知识库现有「仓库无单元测试」约定改为「以编译验证为基线 + 关键逻辑/宏/组件有针对性测试」。

## Capabilities

### New Capabilities
- `test-suite`: 全库测试策略契约——三层（宏 trybuild / 运行时单测 / 组件渲染）各自的覆盖范围、测试放置位置、失败用例只断言稳定报错的原则、单次离屏渲染 harness 的约定、以及随 `cargo test` 运行不改 CI。

### Modified Capabilities
<!-- 无：openspec/specs/ 当前为空。 -->

## Impact

- **代码**：`packages/ratatui-kit/Cargo.toml` 加 dev-dependency `trybuild`；新增 `tests/ui.rs` + `tests/ui/{pass,fail}/`；各模块新增 `#[cfg(test)] mod tests`；新增 `#[cfg(test)]`（或 test-only）的单次渲染 harness（建树→一次 update+draw→读 `TestBackend` Buffer）。
- **依赖关系**：`router` 路径匹配的单测在 [`cache-router-regex`] 变更把匹配逻辑抽成可测函数后更易写——建议**先 apply `cache-router-regex` 再补 router 匹配单测**（或经渲染 harness 间接测）。
- **特性门控**：`routes!`/`use_stores!`/`#[derive(Store)]`/`stateful` 等需在对应 feature 下，CI 已 `--all-features`。
- **CI**：现有 `cargo test ... --tests` 自动带跑；无需改 CI/lefthook。
- **依赖**：新增 dev-dependency `trybuild`（仅测试期）；`TestBackend` 复用 ratatui 自带。
- **非破坏性**（仅加测试 + 可能的 test-only 渲染 helper）。
