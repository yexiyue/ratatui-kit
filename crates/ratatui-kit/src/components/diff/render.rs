use ratatui::{
    style::{Color, Style},
    text::{Line, Span, Text},
};

use super::compute::{DiffLine, DiffTag};

/// Diff 渲染配色主题。
#[derive(Debug, Clone)]
pub struct DiffTheme {
    /// 新增行文字颜色
    pub add_fg: Color,
    /// 新增行背景色
    pub add_bg: Color,
    /// 删除行文字颜色
    pub remove_fg: Color,
    /// 删除行背景色
    pub remove_bg: Color,
    /// 行号颜色
    pub line_num: Color,
    /// 未修改行颜色
    pub unchanged: Color,
}

impl Default for DiffTheme {
    fn default() -> Self {
        Self {
            add_fg: Color::Green,
            add_bg: Color::Rgb(20, 40, 20),
            remove_fg: Color::Red,
            remove_bg: Color::Rgb(40, 20, 20),
            line_num: Color::DarkGray,
            unchanged: Color::Gray,
        }
    }
}

/// 将 diff 行列表渲染为 ratatui Text。
pub fn render_diff(
    lines: &[DiffLine],
    show_line_numbers: bool,
    theme: &DiffTheme,
) -> Text<'static> {
    lines
        .iter()
        .map(|line| {
            let mut spans = Vec::new();

            let (prefix, fg, bg) = match line.tag {
                DiffTag::Insert => ("+", theme.add_fg, theme.add_bg),
                DiffTag::Delete => ("-", theme.remove_fg, theme.remove_bg),
                DiffTag::Unchanged => (" ", theme.unchanged, Color::Reset),
            };

            if show_line_numbers {
                let old_str = line
                    .old_line_num
                    .map(|n| format!("{:>3} ", n))
                    .unwrap_or_else(|| "    ".to_string());
                let new_str = line
                    .new_line_num
                    .map(|n| format!("{:>3} ", n))
                    .unwrap_or_else(|| "    ".to_string());
                spans.push(Span::styled(
                    format!("{}{}{}", prefix, old_str, new_str),
                    Style::new().fg(theme.line_num).bg(bg),
                ));
            } else {
                spans.push(Span::styled(
                    format!("{} ", prefix),
                    Style::new().fg(fg).bg(bg),
                ));
            }

            let content = line.content.trim_end_matches('\n');
            if let Some(ref word_diffs) = line.word_diffs {
                // 单词级 diff：Delete 行只显示 Unchanged + Delete，Insert 行只显示 Unchanged + Insert
                let skip_tag = match line.tag {
                    DiffTag::Delete => Some(DiffTag::Insert),
                    DiffTag::Insert => Some(DiffTag::Delete),
                    DiffTag::Unchanged => None,
                };
                for wd in word_diffs {
                    if Some(wd.tag) == skip_tag {
                        continue;
                    }
                    let (w_fg, w_bg) = match wd.tag {
                        DiffTag::Insert => (theme.add_fg, theme.add_bg),
                        DiffTag::Delete => (theme.remove_fg, theme.remove_bg),
                        DiffTag::Unchanged => (theme.unchanged, bg),
                    };
                    spans.push(Span::styled(
                        wd.text.clone(),
                        Style::new().fg(w_fg).bg(w_bg),
                    ));
                }
            } else {
                spans.push(Span::styled(
                    content.to_string(),
                    Style::new().fg(fg).bg(bg),
                ));
            }

            Line::from(spans)
        })
        .collect::<Vec<_>>()
        .into()
}
