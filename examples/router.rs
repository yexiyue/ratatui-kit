use ratatui::{
    style::{Style, Stylize},
    text::Line,
};
use ratatui_kit::{
    crossterm::event::KeyEvent,
    ratatui::{self, layout::Direction},
};
use ratatui_kit::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    ratatui::layout::Constraint,
};
use std::fs;

#[tokio::main]
async fn main() {
    let routes = routes! {
        "/" => HomePage,
        "/counter" => CounterPage,
        "/markdown" => MarkdownReader,
        "/input" => InputPage,
    };

    element!(RouterProvider(
        routes:routes,
        index_path:"/",
    ))
    .into_any()
    .fullscreen()
    .await
    .expect("Failed to run the application");
}

#[component]
fn HomePage(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut navigate = hooks.use_navigate();

    hooks.use_events(move |event| {
        if let Event::Key(key_event) = event {
            if key_event.kind == KeyEventKind::Press {
                match key_event.code {
                    KeyCode::Char('1') => navigate.push("/counter"),
                    KeyCode::Char('2') => navigate.push("/markdown"),
                    KeyCode::Char('3') => navigate.push("/input"),
                    _ => {}
                }
            }
        }
    });

    element!(
        Fragment{
            Border(
                style:Style::default().blue(),
                height:Constraint::Length(8),
                top_title:Line::from("🏠 Home - 多页面路由示例").centered().bold(),
            ){
                $Line::from("1. 计数器页面 (Counter)")
                $Line::from("2. Markdown 阅读器")
                $Line::from("3. 文本输入页面")
            }
        }
    )
}

#[component]
fn CounterPage(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut state = hooks.use_state(|| 0);
    let mut navigate = hooks.use_navigate();
    hooks.use_future(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            state += 1;
        }
    });
    hooks.use_events(move |event| {
        if let Event::Key(key_event) = event {
            if key_event.kind == KeyEventKind::Press && key_event.code == KeyCode::Esc {
                navigate.back();
            }
        }
    });
    element!(
        Border(
            style:Style::default().green(),
            height:Constraint::Length(5),
            top_title:Line::from("计数器页面 (ESC 返回)").centered(),
        ){
            $Line::styled(
                format!("Counter: {state}"),
                Style::default().fg(ratatui::style::Color::Green).bold(),
            ).centered().bold().underlined()
        }
    )
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
    let mut navigate = hooks.use_navigate();

    let scroll_view_state = hooks.use_state(ScrollViewState::default);
    hooks.use_local_events(move |event| match event {
        Event::Key(KeyEvent {
            kind: KeyEventKind::Press,
            code: KeyCode::Esc,
            ..
        }) => {
            navigate.back();
        }
        _ => {
            scroll_view_state.write().handle_event(&event);
        }
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
            Border(
                border_style:Style::default().blue(),
                top_title:Some(Line::from("Markdown 阅读器 (ESC 返回)").centered()),
                bottom_title:Some(Line::from("上下/翻页滚动，Ctrl+C 退出").centered()),
            ){
                ScrollView(
                    flex_direction:Direction::Vertical,
                    scroll_view_state: scroll_view_state.get(),
                ){
                    #(rendered_elements)
                }
            }
        }
    )
}

#[component]
fn InputPage(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut value = hooks.use_state(String::new);

    let mut navigate = hooks.use_navigate();

    hooks.use_events(move |event| {
        if let Event::Key(key_event) = event {
            if key_event.kind == KeyEventKind::Press && key_event.code == KeyCode::Esc {
                navigate.back();
            }
        }
    });
    element!(
        Border(
            style:Style::default().cyan(),
            height:Constraint::Length(6),
            top_title:Line::from("文本输入页面 (ESC 返回)").centered(),
        ){
            TextArea(
                value: value.read().to_string(),
                is_focus: true,
                on_change: move |new_value: String| {
                    value.set(new_value);
                },
                multiline: true,
                cursor_style: Style::default().on_cyan(),
                placeholder: Some("请输入内容...".to_string()),
                placeholder_style: Style::default().cyan(),
            )
        }
    )
}
