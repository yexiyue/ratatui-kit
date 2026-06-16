//! AlertModal 内置组件示例。
//!
//! 按 `a` 打开提示弹窗；弹窗打开时会消费键盘事件，背景列表不会继续移动。

use ratatui_kit::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    ratatui::{
        layout::{Constraint, Direction, Flex},
        style::{Style, Stylize},
        text::Line,
        widgets::Padding,
    },
};

const WORKSPACES: [&str; 5] = [
    "Runtime checks",
    "Docs preview",
    "Release notes",
    "Fixture sync",
    "Publish queue",
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
    let mut alert_open = hooks.use_state(|| false);
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
            KeyCode::Char('j') | KeyCode::Down if selected.get() + 1 < WORKSPACES.len() => {
                selected += 1;
                status.set(format!("background moved: {}", WORKSPACES[selected.get()]));
            }
            KeyCode::Char('k') | KeyCode::Up if selected.get() > 0 => {
                selected -= 1;
                status.set(format!("background moved: {}", WORKSPACES[selected.get()]));
            }
            KeyCode::Char('a') => {
                alert_open.set(true);
                status.set(format!("alert opened for {}", WORKSPACES[selected.get()]));
            }
            KeyCode::Char('q') | KeyCode::Char('Q') => exit(),
            _ => return EventResult::Ignored,
        }

        EventResult::Consumed
    });

    let selected_index = selected.get();
    let selected_label = WORKSPACES[selected_index].to_string();
    let close_label = selected_label.clone();
    let mode = if alert_open.get() {
        "alert"
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
                top_title: Line::from(" alert modal ").blue().bold().centered(),
                bottom_title: Line::from(" j/k move | a alert | Enter close | Esc close | q quit ").dark_gray().centered(),
            ) {
                View(
                    flex_direction: Direction::Horizontal,
                    gap: 2,
                ) {
                    Border(
                        width: Constraint::Length(42),
                        flex_direction: Direction::Vertical,
                        border_style: Style::new().cyan(),
                        top_title: Line::from(" workspace list ").cyan().centered(),
                    ) {
                        for (index, workspace) in WORKSPACES.into_iter().enumerate() {
                            WorkspaceRow(
                                label: workspace,
                                active: index == selected_index,
                                key: workspace,
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
                            Text(text: Line::from(if alert_open.get() {
                                "background keys are blocked"
                            } else {
                                "background keys are active"
                            }).centered())
                        }
                    }
                }
            }
            AlertModal(
                open: alert_open.get(),
                width: Constraint::Length(76),
                height: Constraint::Length(8),
                title: Line::from("Workspace is current"),
                message: format!("{selected_label} is already synchronized. Nothing needs to run."),
                close_hint: Line::from("Enter / Esc").centered(),
                style: Style::new().dim(),
                border_style: Style::new().yellow(),
                title_style: Style::new().yellow().bold(),
                message_style: Style::new(),
                padding: Padding::new(2, 2, 1, 1),
                on_close: move |_: ()| {
                    status.set(format!("closed alert for {close_label}"));
                    alert_open.set(false);
                },
            )
        }
    )
}

#[derive(Default, Props)]
struct WorkspaceRowProps {
    label: &'static str,
    active: bool,
}

#[component]
fn WorkspaceRow(props: &WorkspaceRowProps, _hooks: Hooks) -> impl Into<AnyElement<'static>> {
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
