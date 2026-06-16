use std::time::Duration;

use ratatui_kit::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    ratatui::{
        layout::{Constraint, Direction, Flex},
        style::{Style, Stylize},
        text::Line,
    },
};

#[tokio::main]
async fn main() {
    element!(AsyncStateDemo)
        .fullscreen()
        .await
        .expect("Failed to run the application");
}

#[component]
fn AsyncStateDemo(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut refresh = hooks.use_state(|| 0_u64);
    let request_id = refresh.get();

    let result = hooks.use_async_state(
        move || async move {
            tokio::time::sleep(Duration::from_millis(700)).await;

            if request_id % 4 == 3 {
                return Err(format!("request #{} failed", request_id + 1));
            }

            Ok::<_, String>(
                (1..=5)
                    .map(|index| format!("task {} from request #{}", index, request_id + 1))
                    .collect::<Vec<_>>(),
            )
        },
        request_id,
    );

    let mut exit = hooks.use_exit();
    hooks.use_event_handler(EventScope::Current, EventPriority::Normal, move |event| {
        let Event::Key(key) = event else {
            return EventResult::Ignored;
        };
        if key.kind != KeyEventKind::Press {
            return EventResult::Ignored;
        }

        match key.code {
            KeyCode::Char('r') | KeyCode::Char('R') => {
                refresh += 1;
                EventResult::Consumed
            }
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                exit();
                EventResult::Consumed
            }
            _ => EventResult::Ignored,
        }
    });

    let mut lines = vec![
        Line::from(" use_async_state ").cyan().bold().centered(),
        Line::from(format!("request #{:02}", request_id + 1)).centered(),
        Line::from("r refresh | q quit | Ctrl+C exit")
            .dark_gray()
            .centered(),
        Line::from(""),
    ];

    if result.loading.get() {
        lines.push(Line::from("status: loading").yellow().centered());
    } else if result.error.read().is_some() {
        lines.push(Line::from("status: error").red().centered());
    } else {
        lines.push(Line::from("status: ready").green().centered());
    }

    if let Some(error) = result.error.read().as_ref() {
        lines.push(Line::from(format!("error: {error}")).red().centered());
    }

    if let Some(items) = result.data.read().as_ref() {
        for item in items {
            lines.push(Line::from(format!("- {item}")).centered());
        }
    }

    element!(
        Center(
            width: Constraint::Length(62),
            height: Constraint::Length(16),
        ) {
            Border(
                border_style: Style::new().cyan(),
                flex_direction: Direction::Vertical,
                justify_content: Flex::Center,
                top_title: Line::from(" async data flow ").cyan().bold().centered(),
                bottom_title: Line::from(" old data stays visible during refresh ").dark_gray().centered(),
            ) {
                for (index, line) in lines.into_iter().enumerate() {
                    View(height: Constraint::Length(1), key: index) {
                        Text(text: line)
                    }
                }
            }
        }
    )
}
