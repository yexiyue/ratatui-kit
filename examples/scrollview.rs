use ratatui::{
    style::{Style, Stylize},
    text::Line,
};
use ratatui_kit::ratatui::{self, layout::Constraint, widgets::Block};
use ratatui_kit::{prelude::*, ratatui::layout::Direction};
use std::fs;

#[tokio::main]
async fn main() {
    element!(MarkdownReader)
        .fullscreen()
        .await
        .expect("Failed to run the application");
}

#[component]
fn MarkdownReader(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    // 读取 README.md 内容
    let lines = hooks.use_memo(
        || {
            let content = fs::read_to_string("README.md")
                .unwrap_or_else(|_| "无法读取 README.md".to_string());
            content.lines().map(|l| l.to_string()).collect::<Vec<_>>()
        },
        (),
    );

    let scroll_view_state = hooks.use_state(ScrollViewState::default);
    hooks.use_local_events(move |event| {
        scroll_view_state.write().handle_event(&event);
    });

    // 简单 markdown 渲染：标题高亮，其余普通文本
    let rendered: Vec<Line> = lines
        .into_iter()
        .map(|line| {
            if line.starts_with("# ") {
                Line::styled(line, Style::default().yellow().bold())
            } else if line.starts_with("## ") {
                Line::styled(line, Style::default().green().bold())
            } else if line.starts_with("### ") {
                Line::styled(line, Style::default().cyan())
            } else {
                Line::from(line)
            }
        })
        .collect();

    // 渲染每一行为 AnyElement
    let rendered_elements: Vec<AnyElement> = rendered
        .into_iter()
        .map(|line| {
            element!(View(height:Constraint::Length(1)){
                $line
            })
            .into_any()
        })
        .collect();

    element!(
        View(
            flex_direction:ratatui::layout::Direction::Vertical,
            gap:1,
        ){
           ScrollView(
                flex_direction: Direction::Vertical,
                scroll_view_state: scroll_view_state,
                block: Block::bordered()
                    .title(Line::from("Markdown 文件阅读器 (ScrollView 示例) 上下/翻页滚动，Ctrl+C 退出").centered())
                    .border_style(Style::default().blue()),
            ){
                #(rendered_elements)
            }
        }
    )
}
