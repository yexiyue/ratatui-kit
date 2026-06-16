use ratatui_kit::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    ratatui::{
        layout::{Constraint, Direction, Flex},
        style::{Style, Stylize},
        text::Line,
    },
};

const TASKS: [&str; 5] = [
    "Review runtime",
    "Record modal",
    "Write docs",
    "Run checks",
    "Plan next",
];

#[tokio::main]
async fn main() {
    element!(App)
        .fullscreen()
        .await
        .expect("Failed to run the application");
}

#[component]
fn App(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut selected = hooks.use_state(|| 0usize);
    let mut editing = hooks.use_state(|| false);
    let draft = hooks.use_state(String::new);
    let mut confirm_open = hooks.use_state(|| false);
    let mut last_action = hooks.use_state(|| "background list is active".to_string());
    let mut exit = hooks.use_exit();

    hooks.use_event_handler(EventScope::Current, EventPriority::Normal, move |event| {
        let Event::Key(key) = event else {
            return EventResult::Ignored;
        };
        if key.kind != KeyEventKind::Press {
            return EventResult::Ignored;
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') if selected.get() > 0 => {
                selected -= 1;
                last_action.set(format!("background moved: row {}", selected.get() + 1));
            }
            KeyCode::Down | KeyCode::Char('j') if selected.get() + 1 < TASKS.len() => {
                selected += 1;
                last_action.set(format!("background moved: row {}", selected.get() + 1));
            }
            KeyCode::Char('e') => {
                editing.set(true);
                last_action.set("edit layer opened".to_string());
            }
            KeyCode::Char('d') => {
                confirm_open.set(true);
                last_action.set("modal layer opened".to_string());
            }
            KeyCode::Char('q') | KeyCode::Char('Q') => exit(),
            _ => return EventResult::Ignored,
        }

        EventResult::Consumed
    });

    let edit_layer = hooks.use_input_layer(editing.get(), true);
    hooks.use_event_handler(
        EventScope::Layer(edit_layer),
        EventPriority::High,
        move |event| {
            if !editing.get() {
                return EventResult::Ignored;
            }
            let Event::Key(key) = event else {
                return EventResult::Ignored;
            };
            if key.kind != KeyEventKind::Press {
                return EventResult::Ignored;
            }

            match key.code {
                KeyCode::Esc => {
                    editing.set(false);
                    last_action.set("edit cancelled".to_string());
                }
                KeyCode::Enter => {
                    editing.set(false);
                    last_action.set(format!("edit saved: {}", draft.read().as_str()));
                }
                KeyCode::Backspace => {
                    draft.write().pop();
                    last_action.set("edit handled Backspace".to_string());
                }
                KeyCode::Char(c) => {
                    draft.write().push(c);
                    last_action.set(format!("edit captured '{c}'"));
                }
                _ => {}
            }

            EventResult::Consumed
        },
    );

    let modal_layer = hooks.use_input_layer(confirm_open.get(), true);
    hooks.use_event_handler(
        EventScope::Layer(modal_layer),
        EventPriority::High,
        move |event| {
            if !confirm_open.get() {
                return EventResult::Ignored;
            }
            let Event::Key(key) = event else {
                return EventResult::Ignored;
            };
            if key.kind != KeyEventKind::Press {
                return EventResult::Ignored;
            }

            match key.code {
                KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                    confirm_open.set(false);
                    last_action.set("modal cancelled".to_string());
                }
                KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                    confirm_open.set(false);
                    last_action.set("modal confirmed".to_string());
                }
                _ => {
                    last_action.set("modal swallowed background key".to_string());
                }
            }

            EventResult::Consumed
        },
    );

    let selected_index = selected.get();
    let mode = if confirm_open.get() {
        "modal"
    } else if editing.get() {
        "editing"
    } else {
        "background"
    };
    let draft_view = if draft.read().is_empty() {
        "<empty>".to_string()
    } else {
        draft.read().to_string()
    };
    let action_view = last_action.read().to_string();

    element!(
        Center(
            width: Constraint::Length(84),
            height: Constraint::Length(20),
        ) {
            Border(
                flex_direction: Direction::Vertical,
                border_style: Style::new().blue(),
                top_title: Line::from(" input layers ").blue().bold().centered(),
                bottom_title: Line::from(" j/k move | e edit | d dialog | q quit ").dark_gray().centered(),
            ) {
                View(
                    flex_direction: Direction::Horizontal,
                    gap: 2,
                ) {
                    Border(
                        width: Constraint::Length(36),
                        flex_direction: Direction::Vertical,
                        border_style: Style::new().dark_gray(),
                        top_title: Line::from(" background list ").centered(),
                    ) {
                        for (index, task) in TASKS.into_iter().enumerate() {
                            TaskRow(task: task, active: index == selected_index, key: task)
                        }
                    }
                    Border(
                        width: Constraint::Fill(1),
                        flex_direction: Direction::Vertical,
                        justify_content: Flex::Center,
                        border_style: Style::new().cyan(),
                        top_title: Line::from(" active layer ").cyan().centered(),
                    ) {
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from(format!("mode: {mode}")).centered())
                        }
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from(format!("selected: {}", TASKS[selected_index])).centered())
                        }
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from(format!("draft: {draft_view}")).centered())
                        }
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from(action_view).centered())
                        }
                    }
                }
                Modal(
                    open: confirm_open.get(),
                    layer: Some(modal_layer),
                    width: Constraint::Length(44),
                    height: Constraint::Length(7),
                    style: Style::new().dim(),
                ) {
                    Border(
                        flex_direction: Direction::Vertical,
                        justify_content: Flex::Center,
                        border_style: Style::new().yellow(),
                        top_title: Line::from(" confirm action ").yellow().bold().centered(),
                    ) {
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from("The modal owns input now.").centered())
                        }
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from("y confirm | n cancel").centered())
                        }
                    }
                }
            }
        }
    )
}

#[derive(Default, Props)]
struct TaskRowProps {
    task: &'static str,
    active: bool,
}

#[component]
fn TaskRow(props: &TaskRowProps, _hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let marker = if props.active { ">" } else { " " };
    let style = if props.active {
        Style::new().black().on_green()
    } else {
        Style::new()
    };

    element!(
        View(height: Constraint::Length(1)) {
            Text(text: Line::styled(format!("{marker} {}", props.task), style))
        }
    )
}
