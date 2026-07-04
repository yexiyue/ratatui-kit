mod parser;

use std::sync::Arc;

use ratatui::{
    layout::Constraint,
    style::{Color, Style},
    text::{Line, Span},
};
use unicode_width::UnicodeWidthStr;
use ratatui_kit_macros::{Props, component, with_layout_style};

use crate::{
    AnyElement, Hooks, element,
    hooks::UseMemo,
    prelude::{CodeBlock, Divider, Text, View},
};

use super::table::{
    RenderTableRow, Table, TableBorderMode, TableCell, TableCellAlignment, TableColumn,
};

use parser::{ParsedBlock, parse_markdown};

#[with_layout_style]
#[derive(Props, Default)]
pub struct MarkdownProps {
    pub content: String,
    pub children: Vec<AnyElement<'static>>,
}

/// 渲染结果：元素列表 + 总高度
pub struct RenderedMarkdown {
    pub elements: Vec<AnyElement<'static>>,
    /// 内容总行数（含所有间距），用于 ScrollView 精确定位
    pub total_height: u16,
}

#[component]
pub fn Markdown(mut hooks: Hooks, props: &MarkdownProps) -> impl Into<AnyElement<'static>> {
    // 用 use_memo 缓存解析结果，只有 content 变化时才重新解析。
    // render_blocks 每帧调用（开销很小，只遍历 blocks + clone Span）。
    let parsed = hooks.use_memo(
        || parse_markdown(&props.content),
        props.content.clone(),
    );
    let rendered = render_blocks(&parsed.blocks);
    element! {
        View(
            flex_direction: crate::ratatui::layout::Direction::Vertical,
            height: Constraint::Length(rendered.total_height),
        ) {
            { rendered.elements.into_iter() }
        }
    }
}

fn heading_line(level_num: usize, line: &Line<'static>) -> AnyElement<'static> {
    let prefix = "#".repeat(level_num);
    let mut spans = vec![
        Span::styled(prefix, Style::new().fg(Color::DarkGray)),
        Span::raw(" "),
    ];
    spans.extend(line.spans.clone());
    element! {
        View(height: Constraint::Length(1)) {
            Text(text: Line::from(spans))
        }
    }
    .into_any()
}

fn empty_line() -> AnyElement<'static> {
    element! {
        View(height: Constraint::Length(1)) {
            Text(text: Line::from(""))
        }
    }
    .into_any()
}

/// 计算 span 列表的显示宽度。
fn span_width(spans: &[Span<'_>]) -> usize {
    spans.iter().map(|s| s.content.width()).sum()
}

/// 将解析块渲染为 `RenderedMarkdown`（元素列表 + 总高度）。
///
/// `total_height` 即可作为 ScrollView 的内容高度，与渲染输出精确一致。
pub fn render_blocks(blocks: &[ParsedBlock]) -> RenderedMarkdown {
    let mut elements = Vec::new();
    let mut total_height: u16 = 0;
    let mut prev_added_trailing = false;
    let mut prev_was_major = false;

    for block in blocks {
        let is_major = matches!(
            block,
            ParsedBlock::Heading(..) | ParsedBlock::CodeBlock(..)
                | ParsedBlock::Table(..) | ParsedBlock::Rule
        );

        // 上一个 major 块后面没有自动 trailing 空行时，在新 major 块前补空行
        if !prev_added_trailing && prev_was_major && is_major {
            elements.push(empty_line());
            total_height += 1;
        }

        prev_added_trailing = false;

        match block {
            ParsedBlock::Heading(level, line) => {
                let level_num = match level {
                    pulldown_cmark::HeadingLevel::H1 => 1,
                    pulldown_cmark::HeadingLevel::H2 => 2,
                    pulldown_cmark::HeadingLevel::H3 => 3,
                    pulldown_cmark::HeadingLevel::H4 => 4,
                    pulldown_cmark::HeadingLevel::H5 => 5,
                    pulldown_cmark::HeadingLevel::H6 => 6,
                };
                elements.push(heading_line(level_num, line));
                total_height += 1;
                // heading 与下面内容的空行
                elements.push(empty_line());
                total_height += 1;
                prev_added_trailing = true;
            }
            ParsedBlock::Paragraph(lines) => {
                let combined: Vec<Span> = lines.iter().flat_map(|l| l.spans.clone()).collect();
                if combined.iter().all(|s| s.content.is_empty()) {
                    elements.push(empty_line());
                } else {
                    elements.push(
                        element! {
                            View(height: Constraint::Length(1)) {
                                Text(text: Line::from(combined))
                            }
                        }
                        .into_any(),
                    );
                }
                total_height += 1;
            }
            ParsedBlock::CodeBlock(lang, code_lines) => {
                elements.push(empty_line());
                total_height += 1;
                let lang_opt = if lang.is_empty() {
                    None
                } else {
                    Some(lang.clone())
                };
                let line_count = code_lines.len() as u16;
                elements.push(
                    element! {
                        CodeBlock(
                            lines: code_lines.clone(),
                            lang: lang_opt,
                            show_border: false,
                            show_line_numbers: false,
                            height: Constraint::Length(line_count),
                        )
                    }
                    .into_any(),
                );
                total_height += line_count;
                elements.push(empty_line());
                total_height += 1;
                prev_added_trailing = true;
            }
            ParsedBlock::ListItem(item) => {
                let indent = "  ".repeat(item.depth as usize);
                let prefix = if item.ordered {
                    format!("{}{}. ", indent, item.number.unwrap_or(1))
                } else {
                    format!("{}• ", indent)
                };
                let mut spans = vec![Span::styled(prefix, Style::new().fg(Color::DarkGray))];
                spans.extend(item.spans.clone());
                elements.push(
                    element! {
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from(spans))
                        }
                    }
                    .into_any(),
                );
                total_height += 1;
            }
            ParsedBlock::Table(headers, rows, alignments) => {
                let col_count = headers
                    .len()
                    .max(rows.first().map(|r| r.len()).unwrap_or(0));
                if col_count == 0 {
                    elements.push(empty_line());
                    total_height += 1;
                    prev_was_major = true;
                    continue;
                }

                let mut col_widths = vec![0usize; col_count];
                for (i, cell) in headers.iter().enumerate() {
                    col_widths[i] = col_widths[i].max(span_width(cell));
                }
                for row in rows {
                    for (i, cell) in row.iter().enumerate() {
                        if i < col_count {
                            col_widths[i] = col_widths[i].max(span_width(cell));
                        }
                    }
                }
                for w in &mut col_widths {
                    *w = (*w).max(3);
                }

                let columns: Vec<TableColumn> = (0..col_count)
                    .map(|i| {
                        let header = headers
                            .get(i)
                            .map(|spans| Line::from(spans.clone()))
                            .unwrap_or_default();
                        let alignment = match alignments.get(i) {
                            Some(pulldown_cmark::Alignment::Center) => TableCellAlignment::Center,
                            Some(pulldown_cmark::Alignment::Right) => TableCellAlignment::Right,
                            _ => TableCellAlignment::Left,
                        };
                        let width = col_widths[i] as u16;
                        TableColumn::new(header, Constraint::Length(width)).alignment(alignment)
                    })
                    .collect();

                type RowType = Vec<Vec<Span<'static>>>;
                let render_row: RenderTableRow<RowType> = Arc::new(|row, _selected| {
                    row.iter()
                        .map(|cell| TableCell::new(Line::from(cell.clone())))
                        .collect()
                });

                // 表格高度: header(1) + rows + header_sep(1) + row_seps + grid_borders(2)
                let n = rows.len() as u16;
                let table_height = 1 + n + 1 + n.saturating_sub(1) + 2;

                elements.push(
                    element! {
                        Table<RowType>(
                            columns,
                            rows: rows.clone(),
                            render_row: Some(render_row),
                            active: false,
                            border_mode: TableBorderMode::Grid,
                            border_style: Style::new().fg(Color::DarkGray),
                            row_separator: true,
                            height: Constraint::Length(table_height),
                        )
                    }
                    .into_any(),
                );
                total_height += table_height;
            }
            ParsedBlock::Rule => {
                elements.push(
                    element! {
                        View(height: Constraint::Length(1)) {
                            Divider(char: Some('─'), style_cfg: Style::new().fg(Color::DarkGray))
                        }
                    }
                    .into_any(),
                );
                total_height += 1;
            }
        }

        prev_was_major = is_major;
    }

    RenderedMarkdown {
        elements,
        total_height,
    }
}
