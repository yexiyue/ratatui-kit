//! ShortcutInfoModal 内置组件示例。
//!
//! 按 `i` 打开快捷键帮助；弹窗内部可滚动，背景列表不会处理弹窗期间的 `j/k/q`。

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
    let mut shortcuts_open = hooks.use_state(|| false);
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
            KeyCode::Char('j') | KeyCode::Down if selected.get() + 1 < TASKS.len() => {
                selected += 1;
                status.set(format!("moved: {}", TASKS[selected.get()]));
            }
            KeyCode::Char('k') | KeyCode::Up if selected.get() > 0 => {
                selected -= 1;
                status.set(format!("moved: {}", TASKS[selected.get()]));
            }
            KeyCode::Char('i') | KeyCode::Char('I') => {
                shortcuts_open.set(true);
                status.set(format!("opened: {}", TASKS[selected.get()]));
            }
            KeyCode::Char('q') | KeyCode::Char('Q') => exit(),
            _ => return EventResult::Ignored,
        }

        EventResult::Consumed
    });

    let selected_index = selected.get();
    let selected_label = TASKS[selected_index].to_string();
    let close_label = selected_label.clone();
    let mode = if shortcuts_open.get() {
        "shortcuts"
    } else {
        "background"
    };
    let lock_hint = if shortcuts_open.get() {
        "background list is locked"
    } else {
        "background list is active"
    };
    let status_view = status.read().to_string();

    element!(
        Center(
            width: Constraint::Length(94),
            height: Constraint::Length(22),
        ) {
            Border(
                flex_direction: Direction::Vertical,
                gap: 1,
                border_style: Style::new().blue(),
                top_title: Line::from(" shortcut info modal ").blue().bold().centered(),
                bottom_title: Line::from(" j/k move | i shortcuts | j/k scroll inside | Esc close | q quit ").dark_gray().centered(),
            ) {
                View(
                    flex_direction: Direction::Horizontal,
                    gap: 2,
                ) {
                    Border(
                        width: Constraint::Length(42),
                        flex_direction: Direction::Vertical,
                        border_style: Style::new().cyan(),
                        top_title: Line::from(" background list ").cyan().centered(),
                    ) {
                        for (index, task) in TASKS.into_iter().enumerate() {
                            TaskRow(
                                label: task,
                                active: index == selected_index,
                                key: task,
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
                            Text(text: Line::from(lock_hint).centered())
                        }
                    }
                }
            }
            ShortcutInfoModal(
                open: shortcuts_open.get(),
                width: Constraint::Length(78),
                height: Constraint::Length(13),
                title: Line::from("Shortcut reference"),
                close_hint: Line::from("j/k scroll | Esc / i close").centered(),
                sections: shortcut_sections(),
                style: Style::new().dim(),
                border_style: Style::new().yellow(),
                title_style: Style::new().yellow().bold(),
                section_title_style: Style::new().cyan().bold(),
                description_style: Style::new(),
                key_style: Style::new().yellow(),
                on_close: move |_: ()| {
                    status.set(format!("closed: {close_label}"));
                    shortcuts_open.set(false);
                },
            )
        }
    )
}

fn shortcut_sections() -> Vec<ShortcutInfoSection> {
    vec![
        ShortcutInfoSection::new(
            "Navigation",
            [
                ("Move selection down", "j / Down"),
                ("Move selection up", "k / Up"),
                ("Jump to top", "Home"),
                ("Jump to bottom", "End"),
            ],
        ),
        ShortcutInfoSection::new(
            "Modal",
            [
                ("Open shortcut reference", "i"),
                ("Scroll reference", "j / k"),
                ("Page through reference", "PageUp / PageDown"),
                ("Close reference", "Esc / i"),
            ],
        ),
        ShortcutInfoSection::new(
            "Editing",
            [
                ("Start editing", "e"),
                ("Submit draft", "Enter"),
                ("Cancel draft", "Esc"),
                ("Clear draft", "Ctrl+U"),
            ],
        ),
        ShortcutInfoSection::new(
            "Diagnostics",
            [
                ("Run checks", "c"),
                ("Open logs", "l"),
                ("Copy details", "y"),
                ("Refresh data", "r"),
            ],
        ),
        ShortcutInfoSection::new(
            "Application",
            [
                ("Open command palette", ":"),
                ("Toggle help", "?"),
                ("Save workspace", "s"),
                ("Quit", "q"),
            ],
        ),
    ]
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

    element!(
        View(height: Constraint::Length(1)) {
            Text(text: Line::styled(format!("{marker} {}", props.label), style))
        }
    )
}
