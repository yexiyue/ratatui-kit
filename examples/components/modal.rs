//! Modal 内置组件示例。
//!
//! 演示父组件持有 input layer，并把同一个 layer 传给 `Modal`，确保弹窗按键不会穿透背景。

use ratatui_kit::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    ratatui::{
        layout::{Alignment, Constraint, Direction, Flex, Margin},
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
    let mut modal_open = hooks.use_state(|| false);
    let mut background_note = hooks.use_state(|| "background list is active".to_string());
    let mut modal_note = hooks.use_state(|| "modal is closed".to_string());
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
            KeyCode::Char('j') | KeyCode::Down if selected.get() + 1 < TASKS.len() => {
                selected += 1;
                background_note.set(format!("background moved: {}", TASKS[selected.get()]));
                EventResult::Consumed
            }
            KeyCode::Char('k') | KeyCode::Up if selected.get() > 0 => {
                selected -= 1;
                background_note.set(format!("background moved: {}", TASKS[selected.get()]));
                EventResult::Consumed
            }
            KeyCode::Char('m') | KeyCode::Char('M') => {
                modal_open.set(true);
                modal_note.set(format!("modal opened for {}", TASKS[selected.get()]));
                background_note.set("background keys are blocked".to_string());
                EventResult::Consumed
            }
            _ => EventResult::Ignored,
        }
    });

    let layer = hooks.use_input_layer(modal_open.get(), true);
    hooks.use_event_handler(
        EventScope::Layer(layer),
        EventPriority::High,
        move |event| {
            let Event::Key(key) = event else {
                return EventResult::Consumed;
            };
            if key.kind != KeyEventKind::Press {
                return EventResult::Consumed;
            }

            match key.code {
                KeyCode::Enter => {
                    modal_note.set(format!("accepted {}", TASKS[selected.get()]));
                    modal_open.set(false);
                    background_note.set("background list is active".to_string());
                }
                KeyCode::Esc | KeyCode::Char('c') | KeyCode::Char('C') => {
                    modal_note.set("modal closed".to_string());
                    modal_open.set(false);
                    background_note.set("background list is active".to_string());
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    modal_note.set("modal consumed j/down".to_string());
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    modal_note.set("modal consumed k/up".to_string());
                }
                _ => {
                    modal_note.set("modal consumed key".to_string());
                }
            }

            EventResult::Consumed
        },
    );

    let selected_index = selected.get();
    let selected_label = TASKS[selected_index].to_string();
    let background_note_view = background_note.read().to_string();
    let modal_note_view = modal_note.read().to_string();
    let owner = if modal_open.get() {
        "mode: modal"
    } else {
        "mode: background"
    };

    element!(
        Center(
            width: Constraint::Length(96),
            height: Constraint::Length(22),
        ) {
            Border(
                flex_direction: Direction::Vertical,
                gap: 1,
                border_style: Style::new().blue(),
                top_title: Line::from(" modal component ").blue().bold().centered(),
                bottom_title: Line::from(" j/k move | m modal | Enter accept | Esc close | q quit ").dark_gray().centered(),
            ) {
                View(
                    flex_direction: Direction::Horizontal,
                    gap: 2,
                ) {
                    Border(
                        width: Constraint::Length(50),
                        border_style: Style::new().cyan(),
                        top_title: Line::from(" background list ").cyan().centered(),
                    ) {
                        View(
                            flex_direction: Direction::Vertical,
                        ) {
                            for (index, task) in TASKS.into_iter().enumerate() {
                                TaskRow(
                                    label: task,
                                    active: index == selected_index,
                                    key: task,
                                )
                            }
                        }
                    }
                    Border(
                        width: Constraint::Length(34),
                        flex_direction: Direction::Vertical,
                        justify_content: Flex::Center,
                        border_style: Style::new().cyan(),
                        top_title: Line::from(" input owner ").cyan().centered(),
                    ) {
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from(owner).centered())
                        }
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from(format!("selected: {selected_label}")).centered())
                        }
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from(background_note_view).centered())
                        }
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from(modal_note_view).centered())
                        }
                    }
                }
            }
            Modal(
                open: modal_open.get(),
                layer: Some(layer),
                width: Constraint::Length(68),
                height: Constraint::Length(12),
                style: Style::new().dim(),
            ) {
                Border(
                    flex_direction: Direction::Vertical,
                    gap: 1,
                    margin: Margin::new(2, 2),
                    border_style: Style::new().yellow(),
                    top_title: Line::from(" shared input layer ").yellow().bold().centered(),
                    bottom_title: Line::from(" Enter accept | Esc close | j/k stay inside ").dark_gray().centered(),
                ) {
                    View(height: Constraint::Length(1)) {
                        Text(
                            text: Line::from(format!("Task: {selected_label}")),
                            alignment: Alignment::Center,
                        )
                    }
                    View(height: Constraint::Length(1)) {
                        Text(
                            text: Line::from("Modal uses the parent-owned layer."),
                            alignment: Alignment::Center,
                        )
                    }
                    View(height: Constraint::Length(1)) {
                        Text(
                            text: Line::from("Background shortcuts are blocked."),
                            alignment: Alignment::Center,
                        )
                    }
                    View(height: Constraint::Length(1)) {
                        Text(
                            text: Line::from(modal_note.read().to_string()),
                            alignment: Alignment::Center,
                        )
                    }
                }
            }
        }
    )
}

#[derive(Default, Props)]
struct TaskRowProps {
    label: &'static str,
    active: bool,
}

#[component]
fn TaskRow(props: &TaskRowProps, _hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let marker = if props.active { ">" } else { " " };
    let style = if props.active {
        Style::new().black().on_cyan()
    } else {
        Style::new()
    };

    element!(View(height: Constraint::Length(1)) {
        Text(text: Line::from(format!("{marker} {}", props.label)).style(style))
    })
}
