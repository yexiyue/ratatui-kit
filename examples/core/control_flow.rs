//! element! 控制流示例。
//!
//! `element!` 里的 if / if let / for / match 都可以直接返回不同元素类型。

use ratatui_kit::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    ratatui::{
        layout::{Constraint, Direction, Flex},
        style::{Style, Stylize},
        text::Line,
        widgets::Borders,
    },
};

const ROWS: [(&str, &str); 4] = [
    ("if / else", "branch can swap between Text and Border"),
    ("if let", "optional data can render a dedicated branch"),
    ("for", "each row still needs a stable key"),
    ("match", "patterns can return different element shapes"),
];

#[tokio::main]
async fn main() {
    element!(ControlFlowDemo)
        .fullscreen()
        .await
        .expect("Failed to run the application");
}

#[component]
fn ControlFlowDemo(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut count = hooks.use_state(|| 3u8);
    let mut show_name = hooks.use_state(|| true);
    let mut selected = hooks.use_state(|| 0usize);
    let mut exit = hooks.use_exit();

    hooks.use_event_handler(EventScope::Current, EventPriority::Normal, move |event| {
        let Event::Key(key) = event else {
            return EventResult::Ignored;
        };
        if key.kind != KeyEventKind::Press {
            return EventResult::Ignored;
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                exit();
                EventResult::Consumed
            }
            KeyCode::Char('j') | KeyCode::Down => {
                count += 1;
                selected.set((selected.get() + 1).min(ROWS.len() - 1));
                EventResult::Consumed
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if count.get() > 0 {
                    count -= 1;
                }
                selected.set(selected.get().saturating_sub(1));
                EventResult::Consumed
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                show_name.set(!show_name.get());
                EventResult::Consumed
            }
            _ => EventResult::Ignored,
        }
    });

    let count_value = count.get();
    let selected_value = selected.get();
    let maybe_name = show_name.get().then_some("ratatui-kit");

    element!(
        Border(
            top_title: Line::from(" element! control flow ").blue().bold().centered(),
            bottom_title: Line::from(" j/k branch | n optional | q quit ")
                .dark_gray()
                .centered(),
            flex_direction: Direction::Vertical,
            justify_content: Flex::Center,
            gap: 1,
        ) {
            Text(
                text: "j/k count + cursor | n toggle optional name | q quit",
                style: Style::new().dark_gray(),
            )

            if count_value % 2 == 0 {
                Text(
                    text: format!("if branch: count {count_value} is even"),
                    style: Style::new().green().bold(),
                )
            } else {
                Border(borders: Borders::ALL, height: Constraint::Length(3)) {
                    Text(
                        text: format!("else branch: count {count_value} is odd"),
                        style: Style::new().yellow(),
                    )
                }
            }

            if let Some(name) = maybe_name {
                Text(text: format!("if let branch: optional name = {name}"))
            } else if count_value > 4 {
                Text(text: "else if branch: no name, but count is high")
            } else {
                Text(text: "else branch: no name yet")
            }

            Border(
                top_title: Line::from(" for rows ").cyan().centered(),
                height: Constraint::Length(6),
            ) {
                for (index, (label, detail)) in ROWS.iter().enumerate() {
                    Text(
                        key: index,
                        text: format!(
                            "{} {label:<10} {detail}",
                            if index == selected_value { ">" } else { " " },
                        ),
                        style: if index == selected_value {
                            Style::new().black().on_cyan()
                        } else {
                            Style::new()
                        },
                    )
                }
            }

            match selected_value {
                0 => { Text(text: "match branch 0: simple Text") }
                1 => { Border(borders: Borders::LEFT, height: Constraint::Length(3)) {
                    Text(text: "match branch 1: wrapped in a left border")
                } }
                2 => { Text(text: "match branch 2: rows come from a for loop", style: Style::new().cyan()) }
                _ => { Text(text: "match default: every branch can have its own element type") }
            }
        }
    )
}
