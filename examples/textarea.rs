use ratatui_kit::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
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
    let mut value = hooks.use_state(String::new);

    let mut should_exit = hooks.use_state(|| false);

    let mut system_ctx = hooks.use_context_mut::<SystemContext>();
    let insert_before = hooks.use_insert_before();

    if should_exit.get() {
        system_ctx.exit();
    }

    hooks.use_events(move |event| {
        if let Event::Key(key_event) = event {
            if key_event.kind == KeyEventKind::Press {
                match key_event.code {
                    KeyCode::Char('q') => {
                        should_exit.set(true);
                    }
                    KeyCode::Enter => {
                        if !value.read().is_empty() {
                            insert_before
                                .render_before(Line::from(format!("message: {value}")), 1)
                                .finish();

                            value.set(String::new());
                        }
                    }
                    _ => {}
                }
            }
        }
    });

    element!(Border(
        height: Constraint::Length(4),
        style: Style::default().green(),
        bottom_title: Line::styled(
            "Press 'Enter' to submit, 'q' to exit",
            Style::default().yellow(),
        ).centered(),
    ) {
        TextArea(
            value: value.read().to_string(),
            is_focus: true,
            on_change: move |new_value: String| {
                value.set(new_value);
            },
            multiline: false,
            cursor_style: Style::default().on_green(),
            placeholder: Some("Type something...".to_string()),
            placeholder_style: Style::default().green(),
        )

    })
}
