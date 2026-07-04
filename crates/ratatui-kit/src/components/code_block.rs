use ratatui::{
    layout::Alignment,
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Borders, Paragraph, Widget, Wrap},
};
use ratatui_kit_macros::{Props, with_layout_style};

use crate::{AnyElement, Component};

/// 代码块组件。
///
/// 支持可选行号、语言标签。可独立于 Markdown 使用。
/// 启用 feature `highlight` 时会自动进行语法高亮。
///
/// ## 用法示例
/// ```rust,no_run
/// element!(CodeBlock(
///     lines: vec!["fn main() {".to_string(), "    println!(\"hi\");".to_string(), "}".to_string()],
///     lang: Some("rust".to_string()),
///     show_line_numbers: Some(true),
/// ))
/// ```
#[with_layout_style]
#[derive(Props)]
pub struct CodeBlockProps<'a> {
    /// 代码行内容。
    pub lines: Vec<String>,
    /// 编程语言标识（如 "rust", "python"）。用于语法高亮选择。
    pub lang: Option<String>,
    /// 是否显示行号。默认 true。
    pub show_line_numbers: Option<bool>,
    /// 语法高亮主题名。默认 `"base16-ocean.dark"`。
    /// 内置主题: `"base16-ocean.dark"`, `"base16-ocean.light"`,
    /// `"InspiredGitHub"`(亮底), `"Solarized (dark/light)"`,
    /// `"base16-eighties.dark"`, `"base16-mocha.dark"`。
    /// 也可通过 `ThemeSet::add_from_folder` 加载自定义 `.tmTheme` 文件。
    pub highlight_theme: Option<String>,
    /// 行号样式。
    pub line_number_style: Option<Style>,
    /// 代码内容样式（仅在无语法高亮时使用）。
    pub code_style: Option<Style>,
    /// 整体边框样式。
    pub border_style: Option<Style>,
    /// 是否显示外框边框。默认 true。
    pub show_border: Option<bool>,
    /// 子元素（代码块内部不承载子组件，此字段仅为 Props derive 占位）。
    pub children: Vec<AnyElement<'a>>,
}

impl Default for CodeBlockProps<'_> {
    fn default() -> Self {
        Self {
            lines: Vec::new(),
            lang: None,
            show_line_numbers: Some(true),
            show_border: Some(true),
            highlight_theme: Some("base16-ocean.dark".to_string()),
            line_number_style: Some(Style::new().fg(Color::DarkGray)),
            code_style: Some(Style::new().fg(Color::White)),
            border_style: Some(Style::new().fg(Color::DarkGray)),
            children: Vec::new(),
            margin: Default::default(),
            offset: Default::default(),
            width: Default::default(),
            height: Default::default(),
            gap: Default::default(),
            flex_direction: Default::default(),
            justify_content: Default::default(),
        }
    }
}

/// 代码块组件。
pub struct CodeBlock {
    lines: Vec<String>,
    lang: Option<String>,
    show_line_numbers: bool,
    show_border: bool,
    highlight_theme: String,
    line_number_style: Style,
    code_style: Style,
    border_style: Style,
}

impl CodeBlock {
    fn from_props(props: &CodeBlockProps<'_>) -> Self {
        Self {
            lines: props.lines.clone(),
            lang: props.lang.clone(),
            show_line_numbers: props.show_line_numbers.unwrap_or(true),
            show_border: props.show_border.unwrap_or(true),
            highlight_theme: props
                .highlight_theme
                .clone()
                .unwrap_or_else(|| "InspiredGitHub".to_string()),
            line_number_style: props
                .line_number_style
                .unwrap_or(Style::new().fg(Color::DarkGray)),
            code_style: props.code_style.unwrap_or(Style::new().fg(Color::White)),
            border_style: props
                .border_style
                .unwrap_or(Style::new().fg(Color::DarkGray)),
        }
    }

    fn line_number_width(&self) -> u16 {
        if !self.show_line_numbers {
            return 0;
        }
        let max_line = self.lines.len().max(1);
        (max_line.to_string().len() + 1) as u16
    }

    /// 无语法高亮的纯文本渲染。
    fn build_text_plain(&self) -> Text<'static> {
        let line_num_w = self.line_number_width() as usize;

        self.lines
            .iter()
            .enumerate()
            .map(|(i, line_content)| {
                let mut spans = Vec::new();
                if self.show_line_numbers {
                    let num = i + 1;
                    let num_str = format!("{:>width$} ", num, width = line_num_w.saturating_sub(1));
                    spans.push(Span::styled(num_str, self.line_number_style));
                }
                spans.push(Span::styled(line_content.clone(), self.code_style));
                Line::from(spans)
            })
            .collect::<Vec<_>>()
            .into()
    }

    /// 带语法高亮的渲染。仅在 feature `highlight` 开启时可用。
    #[cfg(feature = "highlight")]
    fn build_text_highlighted(&self) -> Text<'static> {
        use std::sync::LazyLock;

        use syntect::easy::HighlightLines;
        use syntect::highlighting::ThemeSet;
        use syntect::parsing::SyntaxSet;
        use syntect::util::LinesWithEndings;

        static SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);
        static THEME_SET: LazyLock<ThemeSet> = LazyLock::new(ThemeSet::load_defaults);

        let syntax = self
            .lang
            .as_deref()
            .and_then(|lang| SYNTAX_SET.find_syntax_by_token(lang));

        let Some(syntax) = syntax else {
            return self.build_text_plain();
        };

        let theme = THEME_SET
            .themes
            .get(self.highlight_theme.as_str())
            .unwrap_or(&THEME_SET.themes["InspiredGitHub"]);
        let mut highlighter = HighlightLines::new(syntax, theme);
        let full_text = self.lines.join("\n");
        let line_num_w = self.line_number_width() as usize;

        let mut result_lines = Vec::new();
        for (i, line) in LinesWithEndings::from(&full_text).enumerate() {
            let mut spans = Vec::new();

            if self.show_line_numbers {
                let num = i + 1;
                let num_str = format!("{:>width$} ", num, width = line_num_w.saturating_sub(1));
                spans.push(Span::styled(num_str, self.line_number_style));
            }

            match highlighter.highlight_line(line, &SYNTAX_SET) {
                Ok(ranges) => {
                    for (style, text) in ranges {
                        let fg =
                            Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b);
                        spans.push(Span::styled(text.to_string(), Style::new().fg(fg)));
                    }
                }
                Err(_) => {
                    let trimmed = line.trim_end_matches('\n');
                    spans.push(Span::styled(trimmed.to_string(), self.code_style));
                }
            }

            result_lines.push(Line::from(spans));
        }

        result_lines.into()
    }

    #[cfg(not(feature = "highlight"))]
    fn build_text_highlighted(&self) -> Text<'static> {
        self.build_text_plain()
    }

    fn build_text(&self) -> Text<'static> {
        self.build_text_highlighted()
    }
}

impl Component for CodeBlock {
    type Props<'a> = CodeBlockProps<'a>;

    fn new(props: &Self::Props<'_>) -> Self {
        Self::from_props(props)
    }

    fn update(
        &mut self,
        props: &mut Self::Props<'_>,
        _hooks: crate::Hooks,
        updater: &mut crate::ComponentUpdater,
    ) {
        *self = Self::from_props(props);
        updater.set_layout_style(props.layout_style());
        updater.update_children(&mut props.children, None);
    }

    fn draw(&mut self, drawer: &mut crate::ComponentDrawer<'_, '_>) {
        let area = drawer.area;

        if self.show_border {
            let block = ratatui::widgets::Block::new()
                .borders(Borders::ALL)
                .border_style(self.border_style);

            let block = if let Some(ref lang) = self.lang {
                let label = format!(" {} ", lang);
                block.title_top(
                    Line::styled(label, Style::new().fg(Color::Cyan)).alignment(Alignment::Left),
                )
            } else {
                block
            };

            let inner = block.inner(area);
            block.render(area, drawer.buffer_mut());

            let text = self.build_text();
            let paragraph = Paragraph::new(text).wrap(Wrap { trim: false });
            paragraph.render(inner, drawer.buffer_mut());
        } else {
            let text = self.build_text();
            let paragraph = Paragraph::new(text).wrap(Wrap { trim: false });
            paragraph.render(area, drawer.buffer_mut());
        }
    }
}
