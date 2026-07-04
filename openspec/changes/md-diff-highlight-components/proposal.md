## Why

ratatui-kit 目前缺少**内容渲染型组件**——Table 是第一个突破，但用户构建文档阅读器、日志查看器、AI 对话界面等应用时，还需要 Markdown 渲染、Diff 对比、代码语法高亮等能力。peri-widgets 已在这些方向有成熟实现（markdown 模块 ~1200 行，diff 模块 ~357 行），但：

- peri 的 markdown 采用**硬编码渲染**（`MarkdownTheme` trait 只能换颜色），无法替换块级元素的渲染组件（类比 react-markdown 的 `components` prop）
- peri 的 diff/markdown/highlight 各自维护独立缓存，模式一致但未统一
- 这些组件「小而独立」，适合 feature flag 门控逐步引入

本变更一次性引入 **3 个 feature-gated 大组件 + 3 个无依赖通用小组件**，统一设计 **MarkdownComponents trait**（类 react-markdown）实现可替换块级渲染。

## What Changes

### 新增 Feature-gated 组件

- **`markdown`**（feature: `markdown = ["pulldown-cmark"]`）:
  完整 Markdown 渲染器。`MarkdownProps` 接收 `content: String` + 可选 `components: Arc<dyn MarkdownComponents>` + 可选 `highlight_theme: String`（需 `markdown-highlight`）。
  核心是 `RenderState` 状态机，将 pulldown-cmark 的 `Event` 流路由为块级结构（段落/标题/代码块/列表/引用/表格/水平线），每种块调 `MarkdownComponents` trait 方法产出 `AnyElement`。
  带 LRU 缓存（216 条目，`(content_hash, width)` key），流式场景靠 content_hash 防抖，每帧重解析+缓存命中即可。

- **`diff`**（feature: `diff = ["similar"]`）:
  Diff 计算 + 渲染组件。`DiffProps` 接收 `old: String` + `new: String` + 可选 `theme: DiffTheme`。
  内部：基于 `similar` 的行级/单词级 diff → LRU 缓存（64 条目）→ 渲染为带行号 + 着色（add/remove/hunk）的 `Text`。
  支持 CJK 对齐填充。

- **`markdown-highlight`**（sub-feature: `markdown-highlight = ["markdown", "syntect"]`）:
  代码块语法高亮。`highlight_code_block(lang, lines) -> Option<Vec<Line>>`。`MarkdownComponents` 默认实现的 `code_block` 方法自动检测此 feature。

### 新增通用小组件（无 feature 依赖）

- **`divider`**（~30 行）: 水平分割线组件。`DividerProps { width, style, char }`。类似 HTML `<hr>`。
- **`blockquote`**（~60 行）: 引用块容器。`BlockquoteProps { depth, children }`。渲染 `▍` 前缀 + 嵌套缩进。
- **`code_block`**（~80 行）: 代码块渲染。`CodeBlockProps { lines, lang, show_line_numbers }`。不需要 markdown feature 即可独立使用。

### 核心设计: `MarkdownComponents` trait

```rust
pub trait MarkdownComponents: Send + Sync + 'static {
    // 块级 → 返回 AnyElement
    fn heading(&self, level: HeadingLevel, spans: Vec<Span<'static>>) -> AnyElement<'static>;
    fn paragraph(&self, spans: Vec<Span<'static>>) -> AnyElement<'static>;
    fn code_block(&self, lang: &str, lines: Vec<String>) -> AnyElement<'static>;
    fn list(&self, items: Vec<ListItem>) -> AnyElement<'static>;
    fn table(&self, headers: Vec<CellContent>, rows: Vec<Vec<CellContent>>) -> AnyElement<'static>;
    fn blockquote(&self, depth: u32, content: Vec<AnyElement<'static>>) -> AnyElement<'static>;
    fn rule(&self) -> AnyElement<'static>;
    // 行内 → 返回 Span（共享同一行）
    fn strong(&self, text: &str) -> Span<'static>;
    fn emphasis(&self, text: &str) -> Span<'static>;
    fn strikethrough(&self, text: &str) -> Span<'static>;
    fn inline_code(&self, text: &str) -> Span<'static>;
    fn link(&self, text: &str, url: &str) -> Span<'static>;
}
```

默认实现 `DefaultMarkdownComponents` 复用已有 Table、Border、Text 等组件。用户可 impl trait 只覆盖关心的块（如自定义表格渲染）。

### 缓存策略

三层统一 LRU 缓存，全部使用 `LazyLock<Mutex<LruCache>>` 全局单例：

| 缓存 | Key | Value | 容量 |
|---|---|---|---|
| MarkdownCache | `(content_hash, max_width)` | `Vec<AnyElement<'static>>` | 256 |
| DiffCache | `(old_hash, new_hash)` | `Vec<DiffHunk>` | 64 |
| HighlightCache | `(lang, content_hash)` | `Vec<Line<'static>>` | 64 |

流式渲染策略：每收到新 chunk 拼接完整 buffer → content_hash 判重 → 命中直接返回缓存 → 未命中全量重解析。不追求增量渲染（pulldown-cmark 需完整输入）。

## Capabilities

### New Capabilities

- `markdown-renderer`: 将 Markdown 文本渲染为 ratatui-kit 组件树，支持自定义块级组件映射
- `diff-viewer`: 行级+单词级 Diff 计算与着色渲染
- `code-highlighting`: syntect 语法高亮
- `dividing-line`: 水平分割线组件
- `quote-container`: 引用块容器组件
- `code-display`: 代码块独立渲染组件

## Impact

- **新文件**: `components/markdown/` (mod, parser, cache, components, inline, ~6 文件)、`components/diff/` (mod, compute, render, cache, ~4 文件)、`components/divider.rs`、`components/blockquote.rs`、`components/code_block.rs`
- **修改文件**: `components/mod.rs` (注册新模块 + feature gate)、`Cargo.toml` (新 dependencies + feature flags)、`lib.rs` prelude 导出
- **新依赖**: `pulldown-cmark` (markdown)、`similar` (diff)、`syntect` (markdown-highlight)、`lru` + `parking_lot` (缓存，或复用 std Mutex)
- **新增 example**: `examples/components/markdown.rs`（覆盖标题/代码块/表格/列表/引用/CJK/自定义组件）、`examples/components/diff_viewer.rs`、`examples/components/code_block.rs`
- **可逆性**: 全部 feature-gated，默认不开启，不影响现有用户
