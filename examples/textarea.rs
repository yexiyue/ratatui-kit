use ratatui_kit::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    ratatui::{
        layout::Constraint,
        style::{Style, Stylize},
    },
};

#[tokio::main]
async fn main() {
    element!(MyTextInput)
        .into_any()
        .fullscreen()
        .await
        .expect("Failed to run the application");
}

#[component]
fn MyTextInput(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut value = hooks.use_state(|| String::new());
    let mut is_focus = hooks.use_state(|| true);
    let mut should_exit = hooks.use_state(|| false);
    let mut system_ctx = hooks.use_context_mut::<SystemContext>();

    if should_exit.get() {
        system_ctx.exit();
    }

    hooks.use_events(move |event| match event {
        Event::Key(key_event) => {
            if key_event.kind == KeyEventKind::Press && key_event.code == KeyCode::Esc {
                is_focus.set(false);
            }
            if key_event.kind == KeyEventKind::Press && key_event.code == KeyCode::Enter {
                is_focus.set(true);
            }
            if key_event.kind == KeyEventKind::Press && key_event.code == KeyCode::Char('q') {
                should_exit.set(true);
            }
        }
        _ => {}
    });

    element!(Border(
        height:Constraint::Length(5),
        style:if is_focus.get() {
            Style::default().green()
        } else {
            Style::default()
        },
        ..Default::default()
    ) {
        TextArea(
            value: value.read().to_string(),
            is_focus:is_focus.get(),
            on_change: move |new_value: String| {
                value.set(new_value);
            },
            multiline: true,
            cursor_style: if is_focus.get() {
                Style::default().on_green()
             } else {
                Style::default()
            },
            placeholder: Some("Type something...".to_string()),
            placeholder_style:  if is_focus.get() {
                Style::default().green()
             } else {
                Style::default().dim()
            },
            ..Default::default(),
        )
    })
}
