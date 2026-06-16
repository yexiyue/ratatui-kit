//! Select 内置组件示例。

use ratatui_kit::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    ratatui::{
        layout::{Constraint, Direction, Flex},
        style::{Color, Style, Stylize},
        text::Line,
    },
};

const ENVIRONMENTS: [&str; 5] = ["Production", "Staging", "Preview", "Development", "Local"];

#[tokio::main]
async fn main() {
    element!(App)
        .fullscreen()
        .await
        .expect("Failed to run the application");
}

#[component]
fn App(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut loaded = hooks.use_state(|| false);
    let mut empty = hooks.use_state(|| false);
    let mut selected = hooks.use_state(|| "Staging");
    let mut message =
        hooks.use_state(|| "loading environments; default cursor will land on Staging".to_string());
    let mut exit = hooks.use_exit();

    hooks.use_future(async move {
        tokio::time::sleep(std::time::Duration::from_millis(600)).await;
        loaded.set(true);
        message.set("ready; default_index applied".to_string());
    });

    hooks.use_event_handler(EventScope::Current, EventPriority::Normal, move |event| {
        if let Event::Key(key) = event
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Char('e') => {
                    let was_empty = empty.get();
                    empty.set(!was_empty);
                    message.set(if was_empty {
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

    let items = if empty.get() || !loaded.get() {
        Vec::new()
    } else {
        ENVIRONMENTS.to_vec()
    };
    let empty_message = if loaded.get() {
        "No environments"
    } else {
        "Loading environments..."
    };
    let mode = if !loaded.get() {
        "loading"
    } else if empty.get() {
        "empty"
    } else {
        "ready"
    };
    let selected_view = selected.get();
    let message_view = message.read().to_string();

    element!(
        Center(
            width: Constraint::Length(84),
            height: Constraint::Length(20),
        ) {
            Border(
                flex_direction: Direction::Vertical,
                border_style: Style::new().fg(Color::Blue),
                top_title: Line::from(" select control ").fg(Color::Blue).bold().centered(),
                bottom_title: Line::from(" j/k move | Enter choose | e empty | q quit ").dark_gray().centered(),
            ) {
                View(
                    flex_direction: Direction::Horizontal,
                    gap: 2,
                ) {
                    Select<&'static str>(
                        width: Constraint::Length(34),
                        items: items,
                        top_title: Line::from(" environment ").centered(),
                        bottom_title: Line::from(" default_index: 1 ").centered(),
                        default_index: Some(1),
                        highlight_symbol: "> ",
                        border_style: Style::new().fg(Color::Cyan),
                        highlight_style: Style::new().fg(Color::Black).bg(Color::Green),
                        empty_style: Style::new().fg(Color::Yellow),
                        empty_message: empty_message,
                        on_select: move |item: &'static str| {
                            selected.set(item);
                            message.set(format!("on_select: {item}"));
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
                            Text(text: Line::from(format!("selected: {selected_view}")).centered())
                        }
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from(message_view).centered())
                        }
                    }
                }
            }
        }
    )
}
