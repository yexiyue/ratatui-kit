//! CodeBlock 内置组件示例。
//!
//! 演示语法高亮、行号切换、主题切换等功能。

use ratatui_kit::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    ratatui::{
        layout::{Constraint, Direction},
        style::{Color, Style, Stylize},
        text::Line,
    },
};

const RUST_CODE: &str = "\
use std::collections::HashMap;

fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 0,
        1 => 1,
        _ => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

fn main() {
    for i in 0..=20 {
        println!(\"fib({}) = {}\", i, fibonacci(i));
    }
}";

const THEMES: &[&str] = &[
    "base16-ocean.dark",
    "base16-ocean.light",
    "base16-eighties.dark",
    "base16-mocha.dark",
    "Solarized (dark)",
    "Solarized (light)",
    "InspiredGitHub",
];

#[tokio::main]
async fn main() {
    element!(App)
        .fullscreen()
        .await
        .expect("Failed to run the application");
}

#[component]
fn App(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut show_numbers = hooks.use_state(|| true);
    let mut theme_idx = hooks.use_state(|| 0usize);
    let mut exit = hooks.use_exit();

    hooks.use_event_handler(EventScope::Current, EventPriority::Normal, move |event| {
        if let Event::Key(key) = event
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Char('n') => {
                    show_numbers.set(!show_numbers.get());
                    return EventResult::Consumed;
                }
                KeyCode::Char('t') | KeyCode::Right => {
                    theme_idx.set((theme_idx.get() + 1) % THEMES.len());
                    return EventResult::Consumed;
                }
                KeyCode::Left => {
                    theme_idx.set(if theme_idx.get() == 0 {
                        THEMES.len() - 1
                    } else {
                        theme_idx.get() - 1
                    });
                    return EventResult::Consumed;
                }
                KeyCode::Char('q') => {
                    exit();
                    return EventResult::Consumed;
                }
                _ => {}
            }
        }
        EventResult::Ignored
    });

    let theme = THEMES[theme_idx.get()];

    element!(
        ScrollView(
            flex_direction: Direction::Vertical,
            scroll_bars: ScrollBars {
                vertical_scrollbar_visibility: ScrollbarVisibility::Automatic,
                ..Default::default()
            },
        ) {
            Border(
                flex_direction: Direction::Vertical,
                gap: 1,
                border_style: Style::new().blue(),
                top_title: Line::from(" code_block component ").blue().bold().centered(),
                bottom_title: Line::from(" n numbers | <- -> theme | q quit ").dark_gray().centered(),
            ) {
                View(height: Constraint::Length(1)) {
                    Text(text: Line::from(format!(
                        "theme: {theme} ({}/{})  |  {}",
                        theme_idx.get() + 1,
                        THEMES.len(),
                        if show_numbers.get() { "numbers: on" } else { "numbers: off" },
                    )))
                }

                CodeBlock(
                    lines: RUST_CODE.lines().map(|s| s.to_string()).collect::<Vec<_>>(),
                    lang: "rust".to_string(),
                    show_line_numbers: show_numbers.get(),
                    highlight_theme: theme.to_string(),
                    border_style: Style::new().fg(Color::Cyan),
                )
            }
        }
    )
}
