use ratatui_kit::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::{tui_input::backend::crossterm::EventHandler, *},
    ratatui::{
        TerminalOptions, Viewport,
        layout::Constraint,
        style::{Style, Stylize},
        text::Line,
    },
};

#[tokio::main]
async fn main() {
    element!(MyTextInput)
        .render_loop(TerminalOptions {
            viewport: Viewport::Inline(4),
        })
        .await
        .expect("Failed to run the application");
}

#[component]
fn MyTextInput(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let value = hooks.use_state(tui_input::Input::default);
    let insert_before = hooks.use_insert_before();

    hooks.use_events(move |event| {
        if let Event::Key(key) = event
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Enter => {
                    if !value.read().value().is_empty() {
                        insert_before
                            .render_before(
                                Line::from(format!("message: {}", value.read().value())),
                                1,
                            )
                            .finish();

                        value.write().reset();
                    }
                }
                _ => {
                    value.write().handle_event(&event);
                }
            }
        }
    });

    element!(Border(
        height: Constraint::Length(4),
        style: Style::default().green(),
        bottom_title: Line::styled(
            "Press 'Enter' to submit, 'Ctrl + C' to exit",
            Style::default().yellow(),
        ).centered(),
    ) {
        Input(
            input: value.read().clone(),
            cursor_style: Style::default().on_green(),
            placeholder: "Type something...".to_string(),
            placeholder_style: Style::default().green(),
            hide_cursor: false,
        )
    })
}
