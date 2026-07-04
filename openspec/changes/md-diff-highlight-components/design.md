## Context

本设计基于对 peri-widgets markdown/diff 模块的逐行分析，将其适配到 ratatui-kit 的组件体系。核心差异：

- **peri**: widget 直接实现 `WidgetRef`/`Widget`，通过 `&mut Frame` 渲染到 buffer。Markdown 用 `RenderState` 协调器直接产出 `Text<'static>`，表格用自绘 buffer。
- **ratatui-kit**: 组件体系是 `Props → Component → Element`，渲染通过 `element!` 宏描述组件树，回调通过 Hooks。表格已有独立 `Table` 组件。

**约束（不可破坏）**:

- 全部 feature-gated，默认不开启，不影响现有用户
- 组件须实现 `Component` trait（`use_hooks` 签名），Props 用 `#[derive(Props)]` + `#[with_layout_style]`
- 遵循 `#[component]` 函数组件或手写 `impl Component` 模式
- 缓存用全局 `LazyLock` 单例（与 peri 一致），但不依赖 `parking_lot`（std `Mutex` 足够，单线程运行时）
- pulldown-cmark 的 Event 流是同步的，不引入异步解析

## Goals / Non-Goals

**Goals:**

- Markdown 支持自定义块级组件映射（`MarkdownComponents` trait），默认实现复用已有 Table/Border/Text
- Diff 行级+单词级计算，带行号渲染
- 语法高亮代码块（独立组件可用 + markdown 内集成）
- Divider/Blockquote/CodeBlock 三个零依赖通用组件
- 三层 LRU 缓存保证流式场景性能
- 流式场景靠 content_hash 防抖 + 全量重解析（不追求增量渲染）
- 每个组件带 example

**Non-Goals:**

- 不做 grapheme-cluster 安全换行（emoji 宽度对齐延后到 unicode-segmentation 独立变更）
- 不做增量 Markdown 解析器（pulldown-cmark 需完整输入，微秒级解析 + 缓存足够快）
- 不做 Markdown 编辑（只读渲染）
- 不做图片渲染（终端图片协议太碎片化）
- 不做 HTML 标签解析（pulldown-cmark 自带 HTML passthrough，简单 strip 即可）

## Decisions

### D1. MarkdownComponents trait → 返回 AnyElement（而非 Text）

peri 的 `MarkdownTheme` 只换颜色，块级结构固定在 coordinator 内部。本设计改为 trait 每个方法返回 `AnyElement<'static>`：

```rust
fn heading(&self, level: HeadingLevel, spans: Vec<Span<'static>>) -> AnyElement<'static>;
fn table(&self, headers: Vec<CellContent>, rows: Vec<Vec<CellContent>>) -> AnyElement<'static>;
```

**好处**:
- 用户可完全替换表格渲染（如用自定义组件替代内置 Table）
- 代码块可集成 syntect 高亮
- 块级结构到组件的映射从 coordinator 中剥离，coordinator 只做「Event → 块级结构」路由

**代价**:
- trait 方法签名更复杂（返回 `AnyElement` 而非 `Line`）
- 默认实现需要构造 Element（但 element! 宏已很好支持）（可通过 `element!` 宏轻松构造）

**否决方案**: 像 peri 一样只暴露 `MarkdownTheme`（颜色 trait）——这只能换颜色，不能换渲染逻辑。这是本设计相比 peri 的最核心改进。

### D2. RenderState 状态机 → ParseResult 中间表示

peri 的 coordinator 内部是 `lines: Vec<Line<'static>>` 直接收集最终输出，代码块缓冲、列表缩进、引用前缀全硬编码进行生成。

本设计改为**先收集块级中间表示，再调 MarkdownComponents 产出 Element**:

```rust
struct ParsedBlock {
    kind: BlockKind, // Heading(level, spans) | Paragraph(spans) | CodeBlock(lang, lines) | ...
}

struct ParseResult {
    blocks: Vec<ParsedBlock>,
}

impl ParseResult {
    fn render(&self, components: &dyn MarkdownComponents) -> Vec<AnyElement<'static>> {
        self.blocks.iter().map(|block| match &block.kind {
            BlockKind::Heading(level, spans) => components.heading(*level, spans.clone()),
            BlockKind::Table(headers, rows) => components.table(headers.clone(), rows.clone()),
            // ...
        }).collect()
    }
}
```

**好处**: 解析与渲染彻底解耦，`ParseResult` 可被缓存（key = `(content_hash, width)`，产出 `Vec<AnyElement<'static>>` 缓存后直接复用）。

**注意**: `AnyElement` 是否 `Clone` 需确认。若不可 Clone，则缓存原始事件流或缓存 `ParseResult` 每次调 `render()`（后者代价很小——只是遍历 blocks + clone Span）。

### D3. Markdown 组件的 Props 设计

```rust
#[derive(Props)]
#[with_layout_style]
pub struct MarkdownProps {
    /// Markdown 源文本
    pub content: String,
    /// 自定义组件映射。默认 DefaultMarkdownComponents
    #[props(default = "Arc::new(DefaultMarkdownComponents)")]
    pub components: Arc<dyn MarkdownComponents>,
    /// 最大渲染宽度（默认 80）。用于表格列宽分配和文本换行
    pub max_width: Option<usize>,
}
```

**Key decisions**:

- `components` 用 `Arc<dyn MarkdownComponents>`（非泛型 `C: MarkdownComponents`）：泛型会污染 `MarkdownProps` 签名、传播到 `element!` 宏和 Component trait 的所有场景，复杂度不可接受。`Arc<dyn trait>` 是 Rust 插件的标准模式。
- `max_width: Option<usize>`：默认从 `use_terminal_size` 推导，但允许显式设置（如在 ScrollView 中）
- 不把 `UseState<bool>` 之类的状态暴露在 Props 上——Markdown 是纯渲染组件，内部状态仅缓存

### D4. 流式渲染策略：完全重新解析 + content_hash 防抖

```
每帧 update:
  buffer.push(new_chunk)
  new_hash = hash(buffer)
  if new_hash == last_hash && same_width:
      return cached_elements  // 缓存命中，零开销
  else:
      elements = parse_and_render(buffer, components, width)
      put_cache(new_hash, width, elements)
      return elements
```

**不追求增量渲染的理由**:

1. pulldown-cmark 需要完整输入才能正确解析（嵌套引用、表格跨行、代码块边界）
2. Markdown 文本通常 < 50KB，pulldown-cmark 解析速度 < 1ms
3. LRU 缓存覆盖重复帧（滚动、resize 等场景）
4. 增量解析器实现复杂度极高（需从头实现 AST differ），收益不匹配

### D5. 缓存架构：三层 LRU

| 缓存 | 位置 | Key | Value | 容量 | 作用 |
|---|---|---|---|---|---|
| MarkdownCache | `markdown/cache.rs` | `(content_hash, max_width)` | `Vec<AnyElement<'static>>` | 256 | 跳过 parse + render 全流程 |
| DiffCache | `diff/cache.rs` | `(old_hash, new_hash)` | `Vec<DiffHunk>` | 64 | 跳过 diff 计算 |
| HighlightCache | `markdown/highlight.rs` | `(lang, content_hash)` | `Vec<Line<'static>>` | 64 | 跳过 syntect 解析 |

全部使用 `std::sync::Mutex`（非 `parking_lot`，减少依赖，单线程下无竞争），全局 `LazyLock` 单例。

**容量选择**: markdown 256（内容量大且变化频繁），diff 64（对比场景复用少），highlight 64（语言种类有限）。

### D6. Table 复用策略

Markdown 的 `DefaultMarkdownComponents::table()` 调用现有 `Table` 组件。需要桥接层将 markdown 的数据结构转为 `TableProps`:

```rust
fn table(&self, headers: Vec<Vec<Span<'static>>>, rows: Vec<Vec<Vec<Span<'static>>>>) -> AnyElement<'static> {
    // header_span/row_span → TableColumn + TableCell → TableProps
    let columns: Vec<TableColumn> = /* ... */;
    let cells: Vec<Vec<TableCell>> = /* ... */;
    element!(Table { columns, rows: cells, active: false, .. })
}
```

**潜在问题**: `Table` 的 `render_row` 是用户自定义闭包，markdown 需要自动生成 `render_row`。方案：`Table` 提供一个 `from_simple_data` 便捷构造器。

### D7. Diff 组件设计

```rust
#[derive(Props)]
#[with_layout_style]
pub struct DiffProps {
    pub old: String,
    pub new: String,
    pub show_line_numbers: Option<bool>, // 默认 true
    pub context_lines: Option<usize>,    // 默认 3，类似 git diff -U
}

// 内部状态
struct DiffState {
    cache: DiffCache,
}
```

渲染输出: `impl Component for Diff` 产出 `Text` Element（多行），每行前缀 `+`/`-`/` ` + 行号 + 颜色。

单词级 diff：连续 Remove+Add 行对内部做 word diff，变更超 40% 跳过（与 peri 一致，防噪声）。

**为什么 Diff 不是返回自定义组件树而是 Text？** Diff 输出高度结构化（每行固定前缀+行号+内容），不需要块级组件替换能力。保持简单。

### D8. CodeBlock 组件（零依赖）

```rust
#[derive(Props)]
#[with_layout_style]
pub struct CodeBlockProps {
    pub lines: Vec<String>,
    pub lang: Option<String>,
    pub show_line_numbers: Option<bool>,
}

// 渲染: Border + 左侧行号 + 代码内容
// 若 feature "markdown-highlight" 开启，自动调 highlight_code_block()
```

独立于 markdown 使用：`element!(CodeBlock { lines: vec!["fn main() {", "    println!(\"hello\");", "}"], lang: "rust" })`

### D9. Blockquote 组件设计

```rust
#[derive(Props)]
#[with_layout_style]
pub struct BlockquoteProps {
    pub depth: Option<u32>,    // 嵌套深度，默认 1
    pub children: Element,
}
```

渲染：`"▍ " * depth` + 缩进的 children。类似 Border 但语义不同（引用而非装饰边框）。

### D10. Divider 组件设计

```rust
#[derive(Props)]
#[with_layout_style]
pub struct DividerProps {
    pub char: Option<char>,   // 默认 '─'
    pub style: Option<Style>, // 默认 dark_gray
}
```

渲染: `char.repeat(available_width)`。30 行，最小最简单的组件。

### D11. Feature Flag 依赖图

```
table ─── unicode-width (已有)

markdown ─── pulldown-cmark
    └── markdown-highlight ─── syntect
    └── 内部复用 table 组件 (通过 MarkdownComponents trait)

diff ─── similar

divider ─── 无依赖
blockquote ─── 无依赖
code_block ─── 无依赖，可选依赖 markdown-highlight
```

### D12. 文件组织结构

```
crates/ratatui-kit/src/components/
├── divider.rs                           # ~30 行
├── blockquote.rs                        # ~60 行
├── code_block.rs                        # ~80 行
├── markdown/
│   ├── mod.rs                           # Markdown 组件 + Props + Component impl
│   ├── parser.rs                        # pulldown-cmark Event → ParseResult
│   ├── components.rs                    # MarkdownComponents trait + Default 实现
│   ├── cache.rs                         # MarkdownCache LRU
│   └── inline.rs                        # 行内样式工具 (Strong/Emphasis/Code/Link → Span)
├── diff/
│   ├── mod.rs                           # Diff 组件 + Props + Component impl
│   ├── compute.rs                       # compute_diff() + compute_word_diff()
│   ├── render.rs                        # render_diff() → Text
│   └── cache.rs                         # DiffCache LRU
└── markdown_highlight/                  # sub-feature
    └── mod.rs                           # HighlightCache + highlight_code_block()
```

## Risks / Tradeoffs

| 风险 | 缓解 |
|---|---|
| `AnyElement` 不可 Clone → 缓存 Vec 不可行 | 缓存 `ParseResult`（纯数据，可 Clone），每帧调 `render()` 便宜（< 1ms） |
| syntect 依赖重（~30MB 编译增量） | feature-gate 完全隔离，默认不编译 |
| pulldown-cmark API 变动 | 锁版本，pulldown-cmark 成熟稳定 |
| MarkdownComponents trait 方法签名随 ratatui 版本变动 | 用 `Span<'static>` / `Line<'static>` 而非借用的 `Span<'a>`，避免生命周期传播 |
| 流式场景每次全量重解析 | content_hash 防抖 + 256 条目 LRU 覆盖绝大部分场景，< 1ms 解析时间可接受 |
