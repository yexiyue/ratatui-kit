# Implementation Tasks

> 分 4 组实施。**第 1 组（小组件）必须先完成**（无依赖风险、验证组件模式），第 2 组（Diff）独立于 Markdown 可并行，第 3 组（Markdown + Highlight）依赖第 1 组和第 2 组的经验，第 4 组收尾。

> 每个组件必须带 example。每组完成后跑 `cargo test --locked --all-features --workspace --lib --tests --examples` + `cargo clippy --all-targets --all-features --workspace -- -D warnings` + `cargo fmt --all --check`。

## 1. 零依赖通用小组件 (Divider + Blockquote + CodeBlock)

- [x] 1.1 新建 `components/divider.rs`:
  `DividerProps { width, height, style, char, direction }`, `#[with_layout_style]`, `impl Component for Divider`。
  渲染: `char.repeat(width)` 或垂直方向 `char` × height 行。
  测试: `#[cfg(test)]` 纯逻辑测试（高度计算、默认值）。

- [x] 1.2 新建 `components/blockquote.rs`:
  `BlockquoteProps { depth, children }`, `#[with_layout_style]`, `impl Component for Blockquote`。
  渲染: 左侧 `"▍ " * depth` 占位 + children 缩进。
  测试: 嵌套渲染测试（depth=1/2/3）。

- [x] 1.3 新建 `components/code_block.rs`:
  `CodeBlockProps { lines, lang, show_line_numbers }`, `#[with_layout_style]`, `impl Component for CodeBlock`。
  渲染: Border 包裹 + 可选行号 + 代码内容。若 feature `markdown-highlight` 开启则调用 `highlight_code_block`。
  测试: 无高亮渲染、行号对齐。

- [x] 1.4 在 `components/mod.rs` 注册: `pub mod divider; pub mod blockquote; pub mod code_block;` 及其 `pub use`。

- [x] 1.5 在 `lib.rs` prelude 导出新组件的 Props 类型。

- [x] 1.6 新建 `examples/components/divider.rs` + 根 `Cargo.toml` 注册 example。演示水平/垂直分割线、自定义字符、样式。

- [x] 1.7 新建 `examples/components/blockquote.rs` + 注册。演示单层/多层嵌套引用、与 Text 组合。

- [x] 1.8 新建 `examples/components/code_block.rs` + 注册。演示行号/无行号、不同语言标签、长行截断。

- [x] 1.9 合入门槛: 四件套全绿。

## 2. Diff 组件

> 依赖 `similar` crate。独立于 Markdown，可并行开发。

- [x] 2.1 新建 `components/diff/mod.rs`:
  `DiffProps { old, new, show_line_numbers, context_lines }`, `#[with_layout_style]`, `impl Component for Diff`。
  update 内: 调 `compute_diff(old, new)` → 渲染为 `Text` Element。
  内部状态: `DiffCache` (LazyLock 全局单例, 64 条目)。

- [x] 2.2 新建 `components/diff/compute.rs`:
  `compute_diff(old, new) -> Vec<DiffLine>`。
  使用 `similar::TextDiff` 做行级 diff。
  `compute_word_diff(remove_line, add_line) -> Option<Vec<WordDiff>>`。
  单词级 diff 仅在连续 Remove+Add 行对中计算，变更超 40% 跳过。

- [x] 2.3 新建 `components/diff/render.rs`:
  `render_diff(diff_lines, show_line_numbers) -> Text<'static>`。
  行前缀 `+`/`-`/` ` + 可选行号 + 内嵌单词级着色。
  CJK 对齐填充。

- [x] 2.4 新建 `components/diff/cache.rs`:
  `DiffCache` 全局单例，key = `(old_hash, new_hash)`, value = `Vec<DiffLine>`。
  `get()` / `put()` 方法。

- [x] 2.5 在 `components/mod.rs` 注册: `#[cfg(feature = "diff")] pub mod diff;`

- [x] 2.6 在 `crates/ratatui-kit/Cargo.toml` 加:
  `similar = { version = "2", optional = true }`
  `[features] diff = ["similar"]`，并在 `full` 中加 `"diff"`。

- [x] 2.7 在 `lib.rs` prelude 导出 `DiffProps`。

- [x] 2.8 新建 `examples/components/diff_viewer.rs` + 注册（`required-features = ["ratatui-kit/diff"]`）。
  演示: 两个版本代码对比、CJK 内容 diff、单词级着色。

- [x] 2.9 纯逻辑测试: `#[cfg(test)] mod tests` 覆盖 compute_diff 边界（空输入/相同内容/全部新增/全部删除/CJK 行）。

- [x] 2.10 合入门槛: 四件套全绿。

## 3. Markdown + Highlight 组件

> 依赖 `pulldown-cmark`、可选 `syntect`。最大最复杂的组件，核心是 `MarkdownComponents` trait + `RenderState` 状态机。

### 3A. 基础设施

- [x] 3A.1 在 `crates/ratatui-kit/Cargo.toml` 加:
  ```toml
  pulldown-cmark = { version = "0.12", optional = true, default-features = false }
  ```
  `[features] markdown = ["pulldown-cmark"]`。
  （先不加 syntect，在 3E 阶段加）

- [x] 3A.2 新建 `components/markdown/mod.rs`:
  `MarkdownProps { content, components, max_width }`, `#[with_layout_style]`, `impl Component for Markdown`。
  `#[component]` 函数组件，内部: hash(content) → 查 MarkdownCache → 命中返回缓存结果 → 未命中调 `parse_and_render()` → 写缓存。

- [x] 3A.3 新建 `components/markdown/components.rs`:
  `MarkdownComponents` trait 定义（见 design.md D1）。
  `DefaultMarkdownComponents` 实现: 每个方法用 `element!` 宏产出现有组件。(heading→Text with bold, paragraph→WrappedText, code_block→CodeBlock, table→Table, blockquote→Blockquote, rule→Divider 等)。

- [x] 3A.4 新建 `components/markdown/inline.rs`:
  行内样式工具: `make_strong_span(text) -> Span`, `make_emphasis_span(text) -> Span`, `make_code_span(text) -> Span`, `make_strikethrough_span(text) -> Span`, `make_link_span(text, url) -> Span`。
  默认样式定义（颜色、修饰符）。

### 3B. 解析器（核心）

- [x] 3B.1 新建 `components/markdown/parser.rs`:
  `ParseResult { blocks: Vec<ParsedBlock> }` 数据结构:
  ```rust
  enum BlockKind {
      Heading(HeadingLevel, Vec<Span<'static>>),
      Paragraph(Vec<Span<'static>>),
      CodeBlock(String /* lang */, Vec<String> /* lines */),
      List(Vec<ListItem>),
      Table(Vec<Vec<Span<'static>>> /* headers */, Vec<Vec<Vec<Span<'static>>>> /* rows */),
      BlockQuote(u32 /* depth */, Vec<ParsedBlock>),
      Rule,
      Blank,
  }
  struct ListItem { ordered: bool, number: Option<u64>, depth: u32, spans: Vec<Span<'static>>, sub_items: Vec<ListItem> }
  ```

- [x] 3B.2 实现 `RenderState` 状态机（参考 peri coordinator.rs 426行，去除硬编码渲染逻辑，改为收集 `ParsedBlock`）:
  - `handle_event(Event)` match 所有 pulldown-cmark 事件类型
  - 行内样式: `Strong`/`Emphasis`/`Strikethrough`/`Code`/`Link` 修改当前 `inline_style`
  - 块级: `Heading`→flush Paragraph + 收集 Heading block; `Paragraph`→收集 span 后 flush; `CodeBlock`→缓冲行; `List`→嵌套栈 + 编号递增; `BlockQuote`→深度栈; `Table`→委托 table 子解析器
  - 去掉 peri 中的 `wrap_osc8` 链接注入（ratatui-kit 不依赖 OSC 8）
  - 去掉 peri 中的 `strip_html_tags`（用默认 pulldown-cmark HTML passthrough 即可）

- [x] 3B.3 实现 `parse_markdown(input, max_width) -> ParseResult`:
  ```rust
  let parser = Parser::new_ext(input, Options::all() - Options::ENABLE_SMART_PUNCTUATION);
  let mut state = RenderState::new(max_width);
  for event in parser { state.handle_event(event); }
  state.finalize()
  ```

- [x] 3B.4 实现 `render_blocks(blocks, components) -> Vec<AnyElement<'static>>`:
  遍历 blocks，match BlockKind，调 components 对应方法。

### 3C. 缓存层

- [x] 3C.1 新建 `components/markdown/cache.rs`:
  `MarkdownCache` 全局单例，容量 256。
  key = `(content_hash: u64, max_width: u16)`, value = `Vec<AnyElement<'static>>`。
  若 `AnyElement` 不可 Clone，value 改为 `ParseResult`（可 Clone，每帧调 `render_blocks`）。

- [x] 3C.2 实现 `get(content, width) -> Option<Vec<AnyElement>>` 和 `put(content, width, elements)`。

### 3D. 模块注册 + 测试

- [x] 3D.1 在 `components/mod.rs` 注册: `#[cfg(feature = "markdown")] pub mod markdown;`

- [x] 3D.2 在 `Cargo.toml` 的 `full` feature 中加 `"markdown"`。

- [x] 3D.3 在 `lib.rs` prelude 导出 `MarkdownProps`、`MarkdownComponents`、`DefaultMarkdownComponents`。

- [x] 3D.4 纯逻辑测试: parser 单测覆盖 标题/段落/代码块/列表/嵌套引用/表格/CJK/空输入/纯文本。

- [x] 3D.5 新建 `examples/components/markdown.rs` + 注册（`required-features = ["ratatui-kit/markdown"]`）。
  内容: 标题 H1-H6、段落、粗斜体、行内代码、链接、有序/无序列表（含嵌套）、引用块（含嵌套）、代码块、表格、水平线、CJK 混排。
  交互: 切换主题、切换自定义组件 vs 默认组件（验证 MarkdownComponents trait 可替换）。

- [x] 3D.6 合入门槛: 四件套全绿。

### 3E. Syntax Highlighting (sub-feature)

- [x] 3E.1 在 `Cargo.toml` 加:
  ```toml
  syntect = { version = "5", optional = true, default-features = false, features = ["parsing", "html"] }
  ```
  `[features] markdown-highlight = ["markdown", "syntect"]`

- [x] 3E.2 新建 `components/markdown/highlight.rs`:
  `HighlightCache` 全局单例，容量 64，key = `(lang, content_hash)`。
  `highlight_code_block(lang: &str, lines: &[String]) -> Option<Vec<Line<'static>>>`。
  使用 syntect 默认主题，加载内置语法定义。

- [x] 3E.3 让 `DefaultMarkdownComponents::code_block()` 在 feature 开启时自动调用 `highlight_code_block()`。
  通过 `#[cfg(feature = "markdown-highlight")]` 条件编译。

- [x] 3E.4 在 markdown example 中加语法高亮代码块（rust/python/bash），验证 feature 组合。

- [x] 3E.5 在 `full` feature 中加 `"markdown-highlight"`。

- [x] 3E.6 合入门槛: 四件套全绿（含 `--all-features` 验证 markdown-highlight 分支）。

## 4. 最终验证收尾

- [x] 4.1 `cargo test --locked --all-features --workspace --lib --tests --examples` 全过。

- [x] 4.2 `cargo clippy --all-targets --all-features --workspace -- -D warnings` 全过。

- [x] 4.3 `cargo fmt --all --check` 全过。

- [x] 4.4 `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items --all-features --workspace --examples` 全过。

- [x] 4.5 逐个 example 手验: divider/blockquote/code_block/diff_viewer/markdown 各路径交互正确。

- [x] 4.6 验证默认 feature 构建不引入新依赖: `cargo build --no-default-features -p ratatui-kit` 仅含 path 依赖 + crossterm + ratatui。

- [x] 4.7 Git 提交: `git add -A && git commit -m "feat: add markdown, diff, highlight, divider, blockquote, code_block components"`。
