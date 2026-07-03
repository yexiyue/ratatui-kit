use ratatui::text::{Line, Span};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use super::types::{TableCellAlignment, TableWrapMode};

pub(super) fn wrap_line(
    line: Line<'static>,
    max_width: usize,
    wrap_mode: TableWrapMode,
    alignment: TableCellAlignment,
) -> Vec<Line<'static>> {
    if max_width == 0 {
        return vec![Line::default()];
    }

    let spans = line.spans;
    let full_text = spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>();
    let style = spans.first().map(|span| span.style).unwrap_or(line.style);

    if full_text.width() <= max_width {
        return vec![align_line(
            Line::styled(full_text, style),
            max_width,
            alignment,
        )];
    }

    if wrap_mode == TableWrapMode::Truncate {
        return vec![align_line(
            Line::styled(truncate_text(&full_text, max_width), style),
            max_width,
            alignment,
        )];
    }

    let mut lines = Vec::new();
    let mut byte_pos = 0;
    while byte_pos < full_text.len() {
        let mut cur_width = 0usize;
        let mut content_end = byte_pos;

        for (i, c) in full_text[byte_pos..].char_indices() {
            let char_width = c.width().unwrap_or(0);
            if content_end > byte_pos && cur_width + char_width > max_width {
                break;
            }
            cur_width += char_width;
            content_end = byte_pos + i + c.len_utf8();
        }

        let mut break_at = content_end;
        for (i, c) in full_text[byte_pos..content_end].char_indices().rev() {
            if c.is_whitespace() {
                break_at = byte_pos + i;
                break;
            }
        }

        if break_at <= byte_pos {
            break_at = content_end;
        }

        let segment = full_text[byte_pos..break_at].trim();
        if !segment.is_empty() {
            lines.push(align_line(
                Line::styled(segment.to_string(), style),
                max_width,
                alignment,
            ));
        }

        byte_pos = break_at;
        while byte_pos < full_text.len() {
            let c = full_text[byte_pos..].chars().next().unwrap();
            if c.is_whitespace() {
                byte_pos += c.len_utf8();
            } else {
                break;
            }
        }
    }

    if lines.is_empty() {
        lines.push(Line::default());
    }
    lines
}

pub(super) fn truncate_text(text: &str, max_width: usize) -> String {
    let mut result = String::new();
    let mut width = 0usize;
    for c in text.chars() {
        let char_width = c.width().unwrap_or(0);
        if width + char_width > max_width {
            break;
        }
        width += char_width;
        result.push(c);
    }
    result
}

fn align_line(
    line: Line<'static>,
    max_width: usize,
    alignment: TableCellAlignment,
) -> Line<'static> {
    let width = line.width();
    let padding = max_width.saturating_sub(width);
    match alignment {
        TableCellAlignment::Left => line,
        TableCellAlignment::Center => {
            let left = padding / 2;
            let right = padding - left;
            let mut spans = vec![Span::raw(" ".repeat(left))];
            spans.extend(line.spans);
            spans.push(Span::raw(" ".repeat(right)));
            Line::from(spans)
        }
        TableCellAlignment::Right => {
            let mut spans = vec![Span::raw(" ".repeat(padding))];
            spans.extend(line.spans);
            Line::from(spans)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_line_handles_cjk_visual_width() {
        let lines = wrap_line(
            Line::from("中文English混排"),
            6,
            TableWrapMode::Wrap,
            TableCellAlignment::Left,
        );

        assert!(lines.len() > 1);
        assert!(lines.iter().all(|line| line.width() <= 6));
    }

    #[test]
    fn truncate_text_does_not_split_cjk_bytes() {
        assert_eq!(truncate_text("中文English", 3), "中");
        assert_eq!(truncate_text("中文English", 4), "中文");
    }
}
