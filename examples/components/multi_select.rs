//! MultiSelect 内置组件示例。

use ratatui_kit::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    ratatui::{
        layout::{Constraint, Direction, Flex},
        style::{Color, Style, Stylize},
        text::Line,
    },
};

const CHECKS: [&str; 5] = ["Format", "Clippy", "Unit tests", "Docs", "Package"];

#[tokio::main]
async fn main() {
    element!(App)
        .fullscreen()
        .await
        .expect("Failed to run the application");
}

#[component]
fn App(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut empty = hooks.use_state(|| false);
    let mut selected_count = hooks.use_state(|| 0usize);
    let mut draft = hooks.use_state(|| "<none>".to_string());
    let mut submitted = hooks.use_state(|| "not submitted".to_string());
    let mut exit = hooks.use_exit();

    hooks.use_event_handler(EventScope::Current, EventPriority::Normal, move |event| {
        if let Event::Key(key) = event
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Char('e') => {
                    let was_empty = empty.get();
                    empty.set(!was_empty);
                    selected_count.set(0);
                    draft.set("<none>".to_string());
                    submitted.set(if was_empty {
                        "list restored".to_string()
                    } else {
                        "empty state opened".to_string()
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

    let items = if empty.get() {
        Vec::new()
    } else {
        CHECKS.to_vec()
    };
    let mode = if empty.get() { "empty" } else { "ready" };
    let selected_count_view = selected_count.get();
    let draft_view = draft.read().to_string();
    let submitted_view = submitted.read().to_string();

    element!(
        Center(
            width: Constraint::Length(88),
            height: Constraint::Length(20),
        ) {
            Border(
                flex_direction: Direction::Vertical,
                border_style: Style::new().fg(Color::Blue),
                top_title: Line::from(" multi select ").fg(Color::Blue).bold().centered(),
                bottom_title: Line::from(" j/k move | Space toggle | Enter submit | e empty | q quit ").dark_gray().centered(),
            ) {
                View(
                    flex_direction: Direction::Horizontal,
                    gap: 2,
                ) {
                    MultiSelect<&'static str>(
                        width: Constraint::Length(36),
                        items: items,
                        top_title: Line::from(" release checks ").centered(),
                        bottom_title: Line::from(" draft selection ").centered(),
                        default_index: Some(0),
                        highlight_symbol: "> ",
                        border_style: Style::new().fg(Color::Cyan),
                        highlight_style: Style::new().fg(Color::Black).bg(Color::Green),
                        selected_item_style: Style::new().fg(Color::Yellow).bold(),
                        empty_style: Style::new().fg(Color::Yellow),
                        empty_message: "No checks",
                        on_change: move |items: Vec<&'static str>| {
                            selected_count.set(items.len());
                            draft.set(selection_label(&items));
                            submitted.set("draft changed".to_string());
                        },
                        on_select: move |items: Vec<&'static str>| {
                            submitted.set(if items.is_empty() {
                                "submitted none".to_string()
                            } else {
                                format!("submitted {} checks", items.len())
                            });
                        },
                    )
                    Border(
                        width: Constraint::Fill(1),
                        flex_direction: Direction::Vertical,
                        justify_content: Flex::Center,
                        border_style: Style::new().fg(Color::Cyan),
                        top_title: Line::from(" state ").fg(Color::Cyan).centered(),
                    ) {
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from(format!("mode: {mode}")).centered())
                        }
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from(format!("selected: {selected_count_view}")).centered())
                        }
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from(format!("draft: {draft_view}")).centered())
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

fn selection_label(items: &[&str]) -> String {
    match items.len() {
        0 => "<none>".to_string(),
        1..=3 => items.join(", "),
        count => format!("{count} checks"),
    }
}
