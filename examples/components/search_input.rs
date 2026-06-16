//! SearchInput 内置组件示例。
//!
//! 按 `s` 进入搜索输入态；输入态打开独占 input layer，背景列表的 `j/k` 不会再移动选中项。

use ratatui_kit::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    ratatui::{
        layout::{Constraint, Direction, Flex},
        style::{Color, Style, Stylize},
        text::Line,
    },
};

const COMMANDS: [&str; 6] = [
    "Open dashboard",
    "Create report",
    "Deploy preview",
    "Review logs",
    "Sync workspace",
    "Ship release",
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
    let mut query = hooks.use_state(String::new);
    let mut submitted = hooks.use_state(|| "not submitted".to_string());
    let mut selected = hooks.use_state(|| 0usize);
    let mut exit = hooks.use_exit();

    hooks.use_event_handler(EventScope::Current, EventPriority::Normal, move |event| {
        if let Event::Key(key) = event
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Char('j') | KeyCode::Down if selected.get() + 1 < COMMANDS.len() => {
                    selected += 1;
                    return EventResult::Consumed;
                }
                KeyCode::Char('k') | KeyCode::Up if selected.get() > 0 => {
                    selected -= 1;
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

    let current_query = query.read().to_string();
    let normalized_query = current_query.to_ascii_lowercase();
    let selected_index = selected.get();
    let selected_label = COMMANDS[selected_index];
    let match_count = COMMANDS
        .iter()
        .filter(|item| {
            normalized_query.is_empty() || item.to_ascii_lowercase().contains(&normalized_query)
        })
        .count();
    let submitted_view = submitted.read().to_string();
    let list_lines: Vec<Line<'static>> = COMMANDS
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let matches_query = normalized_query.is_empty()
                || item.to_ascii_lowercase().contains(&normalized_query);
            let base = if matches_query {
                Style::default()
            } else {
                Style::default().fg(Color::DarkGray)
            };

            if index == selected_index {
                Line::styled(format!("> {item}"), base.black().on_cyan())
            } else {
                Line::styled(format!("  {item}"), base)
            }
        })
        .collect();

    element!(
        Center(
            width: Constraint::Length(92),
            height: Constraint::Length(21),
        ) {
            Border(
                flex_direction: Direction::Vertical,
                gap: 1,
                border_style: Style::new().fg(Color::Blue),
                top_title: Line::from(" search input ").fg(Color::Blue).bold().centered(),
                bottom_title: Line::from(" s search | j/k move | Enter submit | Esc cancel | q quit ").dark_gray().centered(),
            ) {
                SearchInput(
                    width: Constraint::Fill(1),
                    value: query.read().to_string(),
                    placeholder: "Press s to search commands".to_string(),
                    on_change: move |next: String| query.set(next),
                    on_clear: move |_: ()| submitted.set("cleared".to_string()),
                    on_submit: move |value: String| {
                        submitted.set(if value.is_empty() {
                            "submitted empty".to_string()
                        } else {
                            format!("submitted {value}")
                        });
                        true
                    },
                    validate: move |value: String| {
                        if value.len() > 18 {
                            (false, "too long".to_string())
                        } else if value.is_empty() {
                            (true, "type to filter".to_string())
                        } else {
                            (true, format!("{} matches", count_matches(&value)))
                        }
                    },
                    clear_on_escape: true,
                    border_style: Style::new().fg(Color::Cyan),
                    active_border_style: Style::new().fg(Color::Yellow),
                    success_border_style: Style::new().fg(Color::Green),
                    error_border_style: Style::new().fg(Color::Red),
                    cursor_style: Style::new().bg(Color::Yellow),
                )
                View(
                    flex_direction: Direction::Horizontal,
                    gap: 2,
                ) {
                    Border(
                        width: Constraint::Length(42),
                        flex_direction: Direction::Vertical,
                        border_style: Style::new().fg(Color::Cyan),
                        top_title: Line::from(" command list ").fg(Color::Cyan).centered(),
                    ) {
                        for (index, line) in list_lines.into_iter().enumerate() {
                            View(height: Constraint::Length(1), key: index) {
                                Text(text: line)
                            }
                        }
                    }
                    Border(
                        width: Constraint::Fill(1),
                        flex_direction: Direction::Vertical,
                        justify_content: Flex::Center,
                        border_style: Style::new().fg(Color::Cyan),
                        top_title: Line::from(" state ").fg(Color::Cyan).centered(),
                    ) {
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from(format!("selected: {selected_label}")).centered())
                        }
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from(format!("query: {}", query_label(&current_query))).centered())
                        }
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from(format!("matches: {match_count}")).centered())
                        }
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from(format!("submit: {submitted_view}")).centered())
                        }
                    }
                }
            }
        }
    )
}

fn count_matches(query: &str) -> usize {
    let query = query.to_ascii_lowercase();
    COMMANDS
        .iter()
        .filter(|item| item.to_ascii_lowercase().contains(&query))
        .count()
}

fn query_label(query: &str) -> String {
    if query.is_empty() {
        "<empty>".to_string()
    } else {
        query.to_string()
    }
}
