//! Compose a search filter from Input and one page-owned input layer.

use ratatui_kit::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::{tui_input::backend::crossterm::EventHandler, *},
    ratatui::{
        layout::{Constraint, Direction, Flex},
        style::{Color, Style, Stylize},
        text::Line,
    },
};

const PEOPLE: [&str; 8] = [
    "Alice Bobson",
    "Bob Marley",
    "Bobby Fischer",
    "Carol Bob",
    "Bob The Builder",
    "Carla Programming",
    "Debra Casey",
    "Fiona Smith",
];

fn filtered(query: &str) -> Vec<&'static str> {
    let query = query.to_ascii_lowercase();
    PEOPLE
        .iter()
        .copied()
        .filter(|name| query.is_empty() || name.to_ascii_lowercase().contains(&query))
        .collect()
}

#[tokio::main]
async fn main() {
    element!(App)
        .fullscreen()
        .await
        .expect("Failed to run the application");
}

#[component]
fn App(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let input = hooks.use_state(tui_input::Input::default);
    let mut query = hooks.use_state(String::new);
    let mut selected = hooks.use_state(|| 0usize);
    let mut editing = hooks.use_state(|| false);
    let mut chosen = hooks.use_state(|| None::<String>);
    let mut status = hooks.use_state(|| "press s to filter".to_string());
    let mut exit = hooks.use_exit();

    // Outside editing, this root-layer handler owns navigation and shortcuts.
    hooks.use_event_handler(EventScope::Current, EventPriority::Normal, move |event| {
        let Event::Key(key) = event else {
            return EventResult::Ignored;
        };
        if key.kind != KeyEventKind::Press || editing.get() {
            return EventResult::Ignored;
        }

        let visible_len = filtered(query.read().as_str()).len();
        match key.code {
            KeyCode::Char('s') => {
                editing.set(true);
                status.set("filter layer opened".to_string());
            }
            KeyCode::Up | KeyCode::Char('k') if selected.get() > 0 => {
                selected -= 1;
                status.set("background moved selection".to_string());
            }
            KeyCode::Down | KeyCode::Char('j') if selected.get() + 1 < visible_len => {
                selected += 1;
                status.set("background moved selection".to_string());
            }
            KeyCode::Char('q') | KeyCode::Char('Q') => exit(),
            _ => return EventResult::Ignored,
        }

        EventResult::Consumed
    });

    // Text input and candidate navigation share this exclusive layer.
    let filter_layer = hooks.use_input_layer(editing.get(), true);
    hooks.use_event_handler(
        EventScope::Layer(filter_layer),
        EventPriority::High,
        move |event| {
            if !editing.get() {
                return EventResult::Ignored;
            }
            let Event::Key(key) = &event else {
                return EventResult::Consumed;
            };
            if key.kind != KeyEventKind::Press {
                return EventResult::Consumed;
            }

            let visible = filtered(query.read().as_str());
            match key.code {
                KeyCode::Esc => {
                    editing.set(false);
                    status.set("filter layer closed".to_string());
                }
                KeyCode::Enter => {
                    let picked = visible
                        .get(selected.get())
                        .copied()
                        .unwrap_or("no matching person");
                    chosen.set(Some(picked.to_string()));
                    editing.set(false);
                    status.set(format!("picked {picked}"));
                }
                KeyCode::Up => {
                    if selected.get() > 0 {
                        selected -= 1;
                    }
                    status.set("filter layer moved selection".to_string());
                }
                KeyCode::Down => {
                    if selected.get() + 1 < visible.len() {
                        selected += 1;
                    }
                    status.set("filter layer moved selection".to_string());
                }
                _ => {
                    let previous = query.read().to_string();
                    input.write().handle_event(&event);
                    let next = input.read().value().to_string();
                    if next != previous {
                        let match_len = filtered(&next).len();
                        query.set(next);
                        selected.set(selected.get().min(match_len.saturating_sub(1)));
                    }
                    status.set("filter layer captured text".to_string());
                }
            }

            EventResult::Consumed
        },
    );

    let query_view = query.read().to_string();
    let visible = filtered(&query_view);
    let selected_index = selected.get().min(visible.len().saturating_sub(1));
    let selected_name = visible.get(selected_index).copied().unwrap_or("no match");
    let list_items: Vec<(&'static str, Line<'static>)> = visible
        .iter()
        .enumerate()
        .map(|(index, person)| {
            let style = if index == selected_index {
                Style::new().black().on_cyan()
            } else {
                Style::new()
            };
            (
                *person,
                Line::styled(
                    format!(
                        " {} {person}",
                        if index == selected_index { ">" } else { " " }
                    ),
                    style,
                ),
            )
        })
        .collect();
    let chosen_view = chosen.read().as_deref().unwrap_or("none").to_string();
    let status_view = status.read().to_string();
    let mode = if editing.get() {
        "filter layer"
    } else {
        "background"
    };

    element!(
        Center(
            width: Constraint::Length(92),
            height: Constraint::Length(22),
        ) {
            Border(
                flex_direction: Direction::Vertical,
                gap: 1,
                border_style: Style::new().fg(Color::Blue),
                top_title: Line::from(" composed search filter ").fg(Color::Blue).bold().centered(),
                bottom_title: Line::from(" s filter | type text | Up/Down pick | Enter choose | Esc close | q quit ").dark_gray().centered(),
            ) {
                Border(
                    height: Constraint::Length(3),
                    border_style: if editing.get() { Style::new().fg(Color::Yellow) } else { Style::new().fg(Color::Cyan) },
                    top_title: Line::from(if editing.get() { " filter input owns this layer " } else { " press s to open the filter layer " }).centered(),
                ) {
                    Input(
                        input: input.read().clone(),
                        placeholder: "type bob to narrow the people list".to_string(),
                        placeholder_style: Style::new().dark_gray(),
                        cursor_style: Style::new().bg(Color::Yellow),
                        style: Style::new().white(),
                        hide_cursor: !editing.get(),
                    )
                }
                View(
                    flex_direction: Direction::Horizontal,
                    gap: 2,
                ) {
                    Border(
                        width: Constraint::Length(46),
                        flex_direction: Direction::Vertical,
                        border_style: Style::new().fg(Color::Cyan),
                        top_title: Line::from(" filtered people ").fg(Color::Cyan).centered(),
                    ) {
                        if visible.is_empty() {
                            View(height: Constraint::Length(1)) {
                                Text(text: Line::from("no matching people").dark_gray().centered())
                            }
                        } else {
                            for (person, line) in list_items.into_iter() {
                                View(height: Constraint::Length(1), key: person) {
                                    Text(text: line)
                                }
                            }
                        }
                    }
                    Border(
                        width: Constraint::Fill(1),
                        flex_direction: Direction::Vertical,
                        justify_content: Flex::Center,
                        border_style: Style::new().fg(Color::Cyan),
                        top_title: Line::from(" layer state ").fg(Color::Cyan).centered(),
                    ) {
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from(format!("mode: {mode}")).centered())
                        }
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from(format!("query: {}", if query_view.is_empty() { "<empty>" } else { query_view.as_str() })).centered())
                        }
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from(format!("selected: {selected_name}")).centered())
                        }
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from(format!("chosen: {chosen_view}")).centered())
                        }
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from(status_view).dark_gray().centered())
                        }
                    }
                }
            }
        }
    )
}
