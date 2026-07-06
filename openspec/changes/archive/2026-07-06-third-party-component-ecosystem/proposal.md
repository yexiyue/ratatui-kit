## Why

组件目前只能通过「合并进主仓库」的方式进入生态,导致主库 `ratatui-kit` 无限膨胀,且每次 ratatui / 框架 breaking 都要连带维护一大批内置组件,不可持续。当前只有两个待处理 PR(#11 table、#12 markdown 生态),是确立「第三方组件各自发 crate、无需合主仓库」生态的最低成本时机。

已用一个「外部作者视角」的 probe crate 实证:框架公共 API 对第三方组件作者**已基本就绪**(手动 `impl Component`、`#[component]`、`element!`、`#[derive(Props)]`、`#[with_layout_style]`、`use_state`、自定义 `Hook` + `use_hook` 全部可用,架构无中央注册表),**唯一阻塞是一个宏路径 bug**。也就是说,这套生态技术上现在就能走通,缺的是一层稳定契约 + 规范 + 发现机制。

## What Changes

- **[P0]** 修复 `#[with_layout_style]` 宏展开生成裸 `ratatui::layout::...` 路径的 bug(6 处),改为绝对路径 `::ratatui_kit::ratatui::layout::...`,使外部 crate 无需自行依赖 `ratatui` 即可获得标准布局能力。这是解锁整个生态的前置阻塞项。
- **[P0]** 确立并文档化「扩展 API 稳定面(Extension API Surface)」:明确哪些公共项对第三方组件作者承诺 semver;内部实现项(如 `ComponentHelperExt`、`AnyProps`)显式标注 internal / `#[doc(hidden)]`。本阶段只做文档化与标注,**不删除现有 `pub` 项**(避免 breaking)。
- **[P0,可选]** 给 `#[component]` / `element!` / `#[derive(Props)]` / `#[with_layout_style]` 增加 `crate = "..."` 逃生舱,防止使用方 `cargo` rename 依赖时宏路径失效。
- **[P1]** 发布第三方组件作者规范 `COMPONENT_GUIDE.md` + `cargo-generate` 模板仓库 `ratatui-kit-component-template`。
- **[P1]** 确立生态三层结构与命名/发现规范:核心 / 官方扩展(`ratatui-kit-contrib` monorepo)/ 社区独立 crate;统一命名前缀 `ratatui-kit-<name>` + crates.io keyword `ratatui-kit` + `awesome-ratatui-kit` 列表 + 主库 README「Ecosystem」段。
- **[P2]** 落地试点:PR #11 table 合入核心;PR #12 markdown 生态抽成独立 crate `ratatui-kit-markdown` 端到端跑通(以此逼出所有仍缺失的扩展 API)。

不含破坏性变更:`with_layout_style` 路径修复对库内既有用法无影响(库内 `ratatui` 仍可达),外部则由「不可用」变「可用」;稳定面本阶段仅文档化与 `#[doc(hidden)]` 标注,不移除任何现有 `pub` 项。未来若要收窄 API 表面(真正删除/隐藏外部可能已用的项),另立 change 并标注 **BREAKING**。

## Capabilities

### New Capabilities
- `extension-api-surface`: 框架侧对第三方组件作者承诺稳定(遵守 semver)的公共扩展 API 契约,以及过程宏必须使用绝对路径(hygiene)、可在外部 crate 中编译通过的要求。
- `component-authoring`: 第三方组件 / 自定义 hook 作者应遵循的契约与约定(只依赖公共 API、透明布局、feature 门控、panic 文案英文、doctest+example 编译基线、声明兼容版本区间),及配套 `cargo-generate` 模板。
- `ecosystem-conventions`: 生态治理约定——三层结构(核心 / 官方扩展 / 社区)、crate 命名前缀 `ratatui-kit-<name>`、crates.io keyword 与发布流程、`awesome-ratatui-kit` 发现机制。

### Modified Capabilities
<!-- 无:本 change 全部为新增能力。with_layout_style 的宏路径修复归入 extension-api-surface 的宏 hygiene 要求,不修改现有 spec 的既有 requirement。 -->

## Impact

- **代码**:`crates/ratatui-kit-macros/src/with_layout_style.rs`(宏路径修复);可能触及 `crates/ratatui-kit/src/lib.rs`(re-export / `#[doc(hidden)]` 标注)、`crates/ratatui-kit-macros/src/{component,element,props}.rs`(`crate =` 逃生舱)。
- **回归护栏**:四件套 `--all-features`(test/clippy/fmt/doc)+ `crates/ratatui-kit/tests/ui/{pass,fail}` trybuild;新增一个「外部 crate 编译」层面的验证(probe 转正为 UI/集成测试或文档示例)。
- **新增仓库 / 交付物**:`ratatui-kit-contrib`(官方扩展 monorepo)、`ratatui-kit-component-template`(cargo-generate)、`awesome-ratatui-kit`;`COMPONENT_GUIDE.md`、扩展 API 稳定面文档、README「Ecosystem」段。
- **发布 / 依赖**:crates.io keyword/category 约定,复用现有「打 tag → CI publish + git-cliff」流程。
- **PR 处置**:#11 合入主库核心;#12 迁出为 `ratatui-kit-markdown` 独立 crate。
