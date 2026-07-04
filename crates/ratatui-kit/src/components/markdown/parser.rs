use pulldown_cmark::{Alignment, Event, HeadingLevel, Tag, TagEnd};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

// ── 块级元素中间表示 ─────────────────────────────────────────────

/// 解析后的块级元素。
#[derive(Debug, Clone)]
pub enum ParsedBlock {
    /// 标题：级别 + 内容行
    Heading(HeadingLevel, Line<'static>),
    /// 段落：文本行列表
    Paragraph(Vec<Line<'static>>),
    /// 代码块：语言 + 代码行
    CodeBlock(String, Vec<String>),
    /// 有序/无序列表项
    ListItem(ListItemData),
    /// 表格：表头 + 表体 + 对齐方式
    Table(
        Vec<Vec<Span<'static>>>,
        Vec<Vec<Vec<Span<'static>>>>,
        Vec<Alignment>,
    ),
    /// 水平分割线
    Rule,
}

/// 列表项数据。
#[derive(Debug, Clone)]
pub struct ListItemData {
    pub ordered: bool,
    pub number: Option<u64>,
    pub depth: u32,
    pub spans: Vec<Span<'static>>,
}

// ── 解析器状态机 ─────────────────────────────────────────────────

/// 解析完成后的结果。
#[derive(Debug, Clone)]
pub struct ParseResult {
    pub blocks: Vec<ParsedBlock>,
}

/// 解析状态机，将 pulldown-cmark Event 流转换为 ParsedBlock 列表。
pub(crate) struct RenderState {
    blocks: Vec<ParsedBlock>,
    /// 当前正在收集的 spans
    current_spans: Vec<Span<'static>>,
    /// 当前行内样式
    inline_style: Style,
    /// 列表栈
    list_stack: Vec<ListStack>,
    /// 引用块深度
    quote_depth: u32,
    /// 是否在代码块内
    in_code_block: bool,
    /// 代码块语言
    code_block_lang: String,
    /// 代码块行缓冲
    code_lines: Vec<String>,
    /// 表格构建器
    table_alignments: Option<Vec<Alignment>>,
    table_rows: Vec<Vec<Vec<Span<'static>>>>,
    table_current_row: Vec<Vec<Span<'static>>>,
    table_current_cell: Vec<Span<'static>>,
    table_in_head: bool,
    /// 进入链接前的行内样式，退出链接时恢复
    prev_inline_style: Style,
    /// 当前链接的 URL
    current_link_url: Option<String>,
}

struct ListStack {
    list_type: ListType,
}

#[derive(Debug, Clone)]
enum ListType {
    Ordered(u64),
    Unordered,
}

impl RenderState {
    pub fn new() -> Self {
        Self {
            blocks: Vec::new(),
            current_spans: Vec::new(),
            inline_style: Style::default(),
            prev_inline_style: Style::default(),
            current_link_url: None,
            list_stack: Vec::new(),
            quote_depth: 0,
            in_code_block: false,
            code_block_lang: String::new(),
            code_lines: Vec::new(),
            table_alignments: None,
            table_rows: Vec::new(),
            table_current_row: Vec::new(),
            table_current_cell: Vec::new(),
            table_in_head: false,
        }
    }

    /// 将当前收集的 spans 刷新为一个 Paragraph block。
    fn flush_spans(&mut self) {
        if !self.current_spans.is_empty() {
            let line = Line::from(std::mem::take(&mut self.current_spans));
            // 查找上一个 block 是否为 Paragraph，合并
            if let Some(ParsedBlock::Paragraph(lines)) = self.blocks.last_mut() {
                lines.push(line);
            } else {
                self.blocks.push(ParsedBlock::Paragraph(vec![line]));
            }
        }
    }

    /// 处理 pulldown-cmark 事件。
    pub fn handle_event(&mut self, event: Event<'_>) {
        match event {
            // ── 标题 ──
            Event::Start(Tag::Heading { .. }) => {
                self.flush_spans();
                self.inline_style = Style::default()
                    .fg(Color::Rgb(255, 193, 7))
                    .add_modifier(Modifier::BOLD);
            }
            Event::End(TagEnd::Heading(level)) => {
                let line = Line::from(std::mem::take(&mut self.current_spans));
                self.blocks.push(ParsedBlock::Heading(level, line));
                self.inline_style = Style::default();
            }

            // ── 段落 ──
            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => {
                self.flush_spans();
            }

            // ── 代码块 ──
            Event::Start(Tag::CodeBlock(kind)) => {
                self.flush_spans();
                self.in_code_block = true;
                self.code_block_lang = match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(lang) => lang.into_string(),
                    pulldown_cmark::CodeBlockKind::Indented => String::new(),
                };
                self.code_lines.clear();
            }
            Event::End(TagEnd::CodeBlock) => {
                self.in_code_block = false;
                let lines = std::mem::take(&mut self.code_lines);
                // 过滤尾部空行
                let mut end = lines.len();
                while end > 0 && lines[end - 1].is_empty() {
                    end -= 1;
                }
                let lang = std::mem::take(&mut self.code_block_lang);
                self.blocks
                    .push(ParsedBlock::CodeBlock(lang, lines[..end].to_vec()));
            }

            // ── 列表 ──
            Event::Start(Tag::List(start)) => {
                self.flush_spans();
                // 最外层列表前加空行
                if self.list_stack.is_empty() {
                    self.blocks
                        .push(ParsedBlock::Paragraph(Vec::new()));
                }
                let list_type = match start {
                    Some(n) => ListType::Ordered(n),
                    None => ListType::Unordered,
                };
                self.list_stack.push(ListStack { list_type });
            }
            Event::End(TagEnd::List(_)) => {
                self.list_stack.pop();
                // 最外层列表结束后加空行
                if self.list_stack.is_empty() {
                    self.blocks
                        .push(ParsedBlock::Paragraph(Vec::new()));
                }
            }
            Event::Start(Tag::Item) => {
                // 列表前缀由 renderer 负责生成，parser 只收集纯内容 span
            }
            Event::End(TagEnd::Item) => {
                let line = Line::from(std::mem::take(&mut self.current_spans));
                let depth = self.list_stack.len().saturating_sub(1) as u32;
                let (ordered, number) = match self.list_stack.last_mut() {
                    Some(ListStack {
                        list_type: ListType::Ordered(n),
                    }) => {
                        let num = *n;
                        *n += 1;
                        (true, Some(num))
                    }
                    _ => (false, None),
                };
                self.blocks.push(ParsedBlock::ListItem(ListItemData {
                    ordered,
                    number,
                    depth,
                    spans: line.spans.to_vec(),
                }));
            }

            // ── 引用块 ──
            Event::Start(Tag::BlockQuote(_)) => {
                self.flush_spans();
                self.quote_depth += 1;
            }
            Event::End(TagEnd::BlockQuote(_)) if self.quote_depth > 0 => {
                self.quote_depth -= 1;
            }

            // ── 表格 ──
            Event::Start(Tag::Table(alignments)) => {
                self.flush_spans();
                self.table_alignments = Some(alignments);
                self.table_rows = Vec::new();
                self.table_current_row = Vec::new();
                self.table_current_cell = Vec::new();
                self.table_in_head = false;
            }
            Event::End(TagEnd::Table) => {
                let headers = if !self.table_rows.is_empty()
                    && self.table_rows[0].iter().any(|c| !c.is_empty())
                {
                    self.table_rows.remove(0)
                } else {
                    Vec::new()
                };
                let rows = std::mem::take(&mut self.table_rows);
                self.blocks.push(ParsedBlock::Table(
                    headers,
                    rows,
                    std::mem::take(
                        self.table_alignments.as_mut().unwrap_or(&mut Vec::new()),
                    ),
                ));
                self.table_alignments = None;
            }
            Event::Start(Tag::TableHead) => {
                self.table_in_head = true;
            }
            Event::End(TagEnd::TableHead) => {
                self.table_in_head = false;
                if !self.table_current_cell.is_empty() {
                    self.table_current_row
                        .push(std::mem::take(&mut self.table_current_cell));
                }
                if !self.table_current_row.is_empty() {
                    self.table_rows
                        .push(std::mem::take(&mut self.table_current_row));
                }
            }
            Event::Start(Tag::TableRow) => {}
            Event::End(TagEnd::TableRow) => {
                if !self.table_current_cell.is_empty() {
                    self.table_current_row
                        .push(std::mem::take(&mut self.table_current_cell));
                }
                if !self.table_current_row.is_empty() {
                    self.table_rows
                        .push(std::mem::take(&mut self.table_current_row));
                }
            }
            Event::Start(Tag::TableCell) => {}
            Event::End(TagEnd::TableCell) => {
                self.table_current_row
                    .push(std::mem::take(&mut self.table_current_cell));
            }

            // ── 水平线 ──
            Event::Rule => {
                self.flush_spans();
                self.blocks.push(ParsedBlock::Rule);
            }

            // ── 行内样式 ──
            Event::Start(Tag::Strong) => {
                self.inline_style = self.inline_style.add_modifier(Modifier::BOLD);
            }
            Event::End(TagEnd::Strong) => {
                self.inline_style = self.inline_style.remove_modifier(Modifier::BOLD);
            }
            Event::Start(Tag::Emphasis) => {
                self.inline_style = self.inline_style.add_modifier(Modifier::ITALIC);
            }
            Event::End(TagEnd::Emphasis) => {
                self.inline_style = self.inline_style.remove_modifier(Modifier::ITALIC);
            }
            Event::Start(Tag::Strikethrough) => {
                self.inline_style = self.inline_style.add_modifier(Modifier::CROSSED_OUT);
            }
            Event::End(TagEnd::Strikethrough) => {
                self.inline_style = self.inline_style.remove_modifier(Modifier::CROSSED_OUT);
            }

            // ── 链接 ──
            Event::Start(Tag::Link { dest_url, .. }) => {
                self.prev_inline_style = self.inline_style;
                self.inline_style = self
                    .inline_style
                    .fg(Color::Rgb(78, 186, 101))
                    .add_modifier(Modifier::UNDERLINED);
                self.current_link_url = Some(dest_url.into_string());
            }
            Event::End(TagEnd::Link) => {
                self.inline_style = self.prev_inline_style;
                // 链接文本后跟 URL
                if let Some(url) = self.current_link_url.take() {
                    self.current_spans.push(Span::styled(
                        format!(" ({url})"),
                        Style::default().fg(Color::DarkGray),
                    ));
                }
            }

            // ── 行内代码 ──
            Event::Code(text) => {
                let style = Style::default().fg(Color::Rgb(162, 169, 228));
                if self.table_alignments.is_some() {
                    self.table_current_cell
                        .push(Span::styled(text.into_string(), style));
                } else {
                    self.current_spans
                        .push(Span::styled(format!("`{}`", text.into_string()), style));
                }
            }

            // ── 文本 ──
            Event::Text(text) => {
                let text_str = text.into_string();
                if self.in_code_block {
                    for line in text_str.split('\n') {
                        self.code_lines.push(line.to_string());
                    }
                } else if self.table_alignments.is_some() {
                    self.table_current_cell
                        .push(Span::styled(text_str, self.inline_style));
                } else {
                    self.current_spans
                        .push(Span::styled(text_str, self.inline_style));
                }
            }

            // ── 换行 ──
            Event::SoftBreak => {
                self.current_spans.push(Span::raw(" "));
            }
            Event::HardBreak => {
                self.flush_spans();
            }

            // ── HTML ──
            Event::Html(html) | Event::InlineHtml(html) => {
                let stripped = strip_html_tags(&html.into_string());
                if !stripped.trim().is_empty() {
                    self.current_spans
                        .push(Span::styled(stripped, self.inline_style));
                }
            }

            _ => {}
        }
    }

    /// 完成解析，返回最终结果。
    pub fn finalize(mut self) -> ParseResult {
        self.flush_spans();
        ParseResult {
            blocks: self.blocks,
        }
    }
}

/// 剥离 HTML 标签。
fn strip_html_tags(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut in_tag = false;
    for ch in input.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }
    result
}

/// 解析 markdown 文本为 ParseResult。
pub fn parse_markdown(input: &str) -> ParseResult {
    use pulldown_cmark::{Options, Parser};

    let options = Options::all() - Options::ENABLE_SMART_PUNCTUATION;
    let parser = Parser::new_ext(input, options);
    let mut state = RenderState::new();
    for event in parser {
        state.handle_event(event);
    }
    state.finalize()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_input() {
        let result = parse_markdown("");
        assert!(result.blocks.is_empty());
    }

    #[test]
    fn test_plain_text() {
        let result = parse_markdown("hello world");
        assert_eq!(result.blocks.len(), 1);
        assert!(matches!(result.blocks[0], ParsedBlock::Paragraph(_)));
    }

    #[test]
    fn test_heading() {
        let result = parse_markdown("# Title\n");
        assert!(matches!(
            result.blocks[0],
            ParsedBlock::Heading(HeadingLevel::H1, _)
        ));
    }

    #[test]
    fn test_code_block() {
        let result = parse_markdown("```rust\nfn main() {}\n```\n");
        assert!(matches!(result.blocks[0], ParsedBlock::CodeBlock(_, _)));
    }

    #[test]
    fn test_list() {
        let result = parse_markdown("- item1\n- item2\n");
        let list_items: Vec<_> = result
            .blocks
            .iter()
            .filter(|b| matches!(b, ParsedBlock::ListItem(_)))
            .collect();
        assert_eq!(list_items.len(), 2);
    }

    #[test]
    fn test_rule() {
        let result = parse_markdown("---\n");
        assert!(result.blocks.iter().any(|b| matches!(b, ParsedBlock::Rule)));
    }

    #[test]
    fn test_bold_italic() {
        let result = parse_markdown("**bold** and *italic*\n");
        assert!(matches!(result.blocks[0], ParsedBlock::Paragraph(_)));
    }

    #[test]
    fn test_table() {
        let result = parse_markdown("| a | b |\n|---|---|\n| 1 | 2 |\n");
        assert!(
            result
                .blocks
                .iter()
                .any(|b| matches!(b, ParsedBlock::Table(_, _, _)))
        );
    }
}
