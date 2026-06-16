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
    element!(Counter)
        .fullscreen()
        .await
        .expect("Failed to run the application");
}

#[component]
fn Counter(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut count = hooks.use_state(|| 0_u64);
    hooks.use_future(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            count += 1;
        }
    });

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
            width: Constraint::Length(48),
            height: Constraint::Length(9),
        ) {
            Border(
                flex_direction: Direction::Vertical,
                justify_content: Flex::Center,
                border_style: Style::new().cyan(),
                top_title: Line::from(" ratatui-kit counter ").cyan().bold().centered(),
                bottom_title: Line::from(" q quit · Ctrl+C exit ").dark_gray().centered(),
            ){
                View(
                    flex_direction: Direction::Vertical,
                    justify_content: Flex::Center,
                    gap: 1,
                ) {
                    View(height: Constraint::Length(1)){
                        Text(text: Line::styled(
                            format!("Counter: {:02}", count.get()),
                            Style::new().green().bold(),
                        ).centered())
                    }
                    View(height: Constraint::Length(1)){
                        Text(text: Line::from("state writes wake the terminal UI").centered())
                    }
                }
            }
        }
    )
}
