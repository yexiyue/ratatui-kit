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
    element!(HelloWorld)
        .fullscreen()
        .await
        .expect("Failed to run the application");
}

#[component]
fn HelloWorld(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut exit = hooks.use_exit();

    hooks.use_event_handler(EventScope::Current, EventPriority::Normal, move |event| {
        let Event::Key(key) = event else {
            return EventResult::Ignored;
        };
        if key.kind == KeyEventKind::Press
            && matches!(key.code, KeyCode::Char('q') | KeyCode::Char('Q'))
        {
            exit();
            return EventResult::Consumed;
        }

        EventResult::Ignored
    });

    element!(
        Center(
            width: Constraint::Length(42),
            height: Constraint::Length(7),
        ) {
            Border(
                flex_direction: Direction::Vertical,
                justify_content: Flex::Center,
                border_style: Style::new().cyan(),
                top_title: Line::from(" ratatui-kit ").cyan().bold().centered(),
                bottom_title: Line::from(" q quit · Ctrl+C exit ").dark_gray().centered(),
            ) {
                Text(text: Line::from("Hello, World!").green().bold().centered())
            }
        }
    )
}
