//! ConfirmModal 内置组件示例。
//!
//! 按 `d` 打开确认弹窗；弹窗内部使用独占 input layer，背景列表不会处理 `j/k/q`。

use ratatui_kit::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    ratatui::{
        layout::{Constraint, Direction, Flex},
        style::{Style, Stylize},
        text::Line,
    },
};

const RELEASES: [&str; 5] = [
    "Draft changelog",
    "Queued build",
    "Canary deploy",
    "Staging verify",
    "Production release",
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
    let mut confirm_open = hooks.use_state(|| false);
    let mut status = hooks.use_state(|| "background list is active".to_string());
    let mut exit = hooks.use_exit();

    hooks.use_event_handler(EventScope::Current, EventPriority::Normal, move |event| {
        let Event::Key(key) = event else {
            return EventResult::Ignored;
        };
        if key.kind != KeyEventKind::Press {
            return EventResult::Ignored;
        }

        match key.code {
            KeyCode::Char('j') | KeyCode::Down if selected.get() + 1 < RELEASES.len() => {
                selected += 1;
                status.set(format!("background moved: {}", RELEASES[selected.get()]));
            }
            KeyCode::Char('k') | KeyCode::Up if selected.get() > 0 => {
                selected -= 1;
                status.set(format!("background moved: {}", RELEASES[selected.get()]));
            }
            KeyCode::Char('d') => {
                confirm_open.set(true);
                status.set(format!("confirm opened for {}", RELEASES[selected.get()]));
            }
            KeyCode::Char('q') | KeyCode::Char('Q') => exit(),
            _ => return EventResult::Ignored,
        }

        EventResult::Consumed
    });

    let selected_index = selected.get();
    let selected_label = RELEASES[selected_index].to_string();
    let confirm_label = selected_label.clone();
    let cancel_label = selected_label.clone();
    let mode = if confirm_open.get() {
        "modal"
    } else {
        "background"
    };
    let status_view = status.read().to_string();

    element!(
        Center(
            width: Constraint::Length(92),
            height: Constraint::Length(21),
        ) {
            Border(
                flex_direction: Direction::Vertical,
                gap: 1,
                border_style: Style::new().blue(),
                top_title: Line::from(" confirm modal ").blue().bold().centered(),
                bottom_title: Line::from(" j/k move | d delete | Tab switch | Enter choose | n cancel | q quit ").dark_gray().centered(),
            ) {
                View(
                    flex_direction: Direction::Horizontal,
                    gap: 2,
                ) {
                    Border(
                        width: Constraint::Length(42),
                        flex_direction: Direction::Vertical,
                        border_style: Style::new().cyan(),
                        top_title: Line::from(" release queue ").cyan().centered(),
                    ) {
                        for (index, release) in RELEASES.into_iter().enumerate() {
                            ReleaseRow(
                                label: release,
                                active: index == selected_index,
                                key: release,
                            )
                        }
                    }
                    Border(
                        width: Constraint::Fill(1),
                        flex_direction: Direction::Vertical,
                        justify_content: Flex::Center,
                        border_style: Style::new().cyan(),
                        top_title: Line::from(" input owner ").cyan().centered(),
                    ) {
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from(format!("mode: {mode}")).centered())
                        }
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from(format!("selected: {selected_label}")).centered())
                        }
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from(status_view).centered())
                        }
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from(if confirm_open.get() {
                                "background keys are blocked"
                            } else {
                                "background keys are active"
                            }).centered())
                        }
                    }
                }
            }
            ConfirmModal(
                open: confirm_open.get(),
                width: Constraint::Length(70),
                height: Constraint::Length(10),
                title: Line::from("Delete release?"),
                content: format!("Remove {selected_label} from the queue?"),
                confirm_text: "Delete".to_string(),
                cancel_text: "Keep".to_string(),
                style: Style::new().dim(),
                border_style: Style::new().yellow(),
                title_style: Style::new().yellow().bold(),
                content_style: Style::new(),
                button_style: Style::new().gray(),
                selected_button_style: Style::new().yellow().bold(),
                on_confirm: move |_: ()| {
                    status.set(format!("deleted {confirm_label}"));
                    confirm_open.set(false);
                },
                on_cancel: move |_: ()| {
                    status.set(format!("kept {cancel_label}"));
                    confirm_open.set(false);
                },
            )
        }
    )
}

#[derive(Default, Props)]
struct ReleaseRowProps {
    label: &'static str,
    active: bool,
}

#[component]
fn ReleaseRow(props: &ReleaseRowProps, _hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let marker = if props.active { ">" } else { " " };
    let style = if props.active {
        Style::new().black().on_cyan()
    } else {
        Style::new()
    };

    element!(
        View(height: Constraint::Length(1)) {
            Text(text: Line::styled(format!("{marker} {}", props.label), style))
        }
    )
}
