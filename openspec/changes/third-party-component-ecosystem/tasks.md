## 1. P0 — 扩展 API 地基(阻塞项,立即可做)

- [x] 1.1 修复 `crates/ratatui-kit-macros/src/with_layout_style.rs` 第 54-72 行 6 处裸 `ratatui::layout::...`,改为 `::ratatui_kit::ratatui::layout::...`(margin/offset/width/height/flex_direction/justify_content)
- [x] 1.2 回归验证:四件套 `--all-features`(test/clippy `-D warnings`/fmt --check/doc `RUSTDOCFLAGS=-D warnings`)+ `crates/ratatui-kit/tests/ui/{pass,fail}` trybuild 全绿
- [x] 1.3 新增「外部 crate 可编译」验证:把 probe 转正为 `tests/ui/pass` 用例或 contrib 内最小 crate,锁死宏 hygiene 回归
- [x] 1.4 起草「扩展 API 稳定面」文档:枚举承诺 semver 的公共项(Component/ComponentUpdater/ComponentDrawer/Element/AnyElement/ElementKey/NoProps、过程宏、Hooks/Hook/use_hook+内置 hooks、State/LayoutStyle、re-export ratatui/crossterm)
- [x] 1.5 给内部实现项(`ComponentHelperExt`、`AnyProps` 等)加 `#[doc(hidden)]` 或文档 internal 标注,不删除现有 `pub` 项
- [ ] 1.6 (可选)为 `#[component]`/`element!`/`#[derive(Props)]`/`#[with_layout_style]` 增加 `crate = "..."` 逃生舱,防依赖 rename

## 2. P1 — 作者规范 + 模板 + 发现机制

- [ ] 2.1 编写 `COMPONENT_GUIDE.md`:组件契约(只依赖公共 API、透明布局陷阱、feature 门控、panic 文案英文、doctest+example 编译基线、版本区间声明)
- [ ] 2.2 编写命名 + 发布规范:`ratatui-kit-<name>` 前缀、keyword `ratatui-kit`、category、official 标注、打 tag 发布流程
- [ ] 2.3 创建 `ratatui-kit-component-template`(cargo-generate):示例组件 + 自定义 hook + 可运行 example + fmt/clippy/test/doc CI + 打 tag 发布配置(内建透明布局正确示范)
- [ ] 2.4 主库 README 增加「Ecosystem」段,指向 awesome-list 与 keyword 检索
- [ ] 2.5 创建 `awesome-ratatui-kit` 列表仓库骨架
- [ ] 2.6 创建 `ratatui-kit-contrib` monorepo 骨架(workspace + CI + 按 tag 前缀发布)

## 3. P2 — 试点:table 入核心

- [ ] 3.1 合入 PR #11(CI 绿、零验证存活 findings、单独满足 issue #10)
- [ ] 3.2 采纳 #12 对 `table/layout.rs` 的 Outer 模式修复(`column_count+1`,防 Outer 边框越界;markdown 用 Grid 不受影响),折入 #11 或接受 #12 rebase 带入

## 4. P2 — 试点:markdown 迁出为独立 crate(迁移即修 review 实证 bug)

- [ ] 4.1 在 contrib 建 `ratatui-kit-markdown` crate,搬入 markdown/code_block/diff/blockquote/divider 源码,依赖改为 crates.io `ratatui-kit` 版本区间
- [ ] 4.2 修 **blocker**:连续段落合并成一行(`parser.rs` flush_spans 把每次 flush 并入前一 Paragraph)——按行分行渲染 + 段间空行 + 硬换行成真换行,加回归测试
- [ ] 4.3 修 **major**:嵌套列表父项丢 bullet + 多余空 bullet(子列表 Start 时把父项已收集 spans 作为 ListItem 发出,跳过空 ListItem),加回归测试
- [ ] 4.4 修 **CI blocker**:删 `examples/components/markdown_streaming.rs` 6 处对 Copy 类型(State/ReactiveHandle)的 `.clone()`,并 `cargo fmt --all`(markdown/mod.rs、parser.rs、markdown_streaming.rs)
- [ ] 4.5 修 minor:标题内 bold/italic 样式泄漏(改用 save/restore 样式栈,同 link 路径);代码块与换行表格行的预留高度按 wrap 感知计算(避免 ScrollView 裁剪);Divider 透明布局属性转发到根元素;补齐 4 处 `///` doctest 缺失的 prelude import
- [ ] 4.6 对齐 openspec 文档(#12 自带 md-diff-highlight-components change:MarkdownComponents trait / 三层 LRU 缓存等描述与实现不符,勾选状态与实现对齐或改写)
- [ ] 4.7 端到端验证:markdown crate 全部 example 跑通 + 编译基线 + 全量四件套,然后发布首个版本

## 5. 收尾

- [ ] 5.1 回填 markdown 迁移中暴露的缺失扩展 API 到稳定面文档
- [ ] 5.2 `openspec archive third-party-component-ecosystem`
