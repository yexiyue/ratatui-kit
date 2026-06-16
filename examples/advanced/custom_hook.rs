//! 自定义 Hook 示例。
//!
//! `use_command_palette` 把查询、过滤、游标夹紧和键盘处理收进一个组合型 hook。

use ratatui_kit::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    ratatui::{
        layout::{Constraint, Direction, Flex},
        style::{Color, Style, Stylize},
        text::Line,
    },
};

#[derive(Clone, Copy, Default)]
struct Command {
    title: &'static str,
    group: &'static str,
    shortcut: &'static str,
}

const COMMANDS: [Command; 8] = [
    Command {
        title: "Deploy API",
        group: "deploy",
        shortcut: "d a",
    },
    Command {
        title: "Deploy Docs",
        group: "deploy",
        shortcut: "d d",
    },
    Command {
        title: "Run Checks",
        group: "test",
        shortcut: "r c",
    },
    Command {
        title: "Open Search",
        group: "nav",
        shortcut: "o s",
    },
    Command {
        title: "Sync Atom Cache",
        group: "state",
        shortcut: "s a",
    },
    Command {
        title: "Record VHS",
        group: "docs",
        shortcut: "r v",
    },
    Command {
        title: "Inspect Layers",
        group: "input",
        shortcut: "i l",
    },
    Command {
        title: "Reset Router",
        group: "router",
        shortcut: "r r",
    },
];

#[tokio::main]
async fn main() {
    element!(CustomHookApp)
        .fullscreen()
        .await
        .expect("Failed to run the application");
}

#[component]
fn CustomHookApp(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let palette = use_command_palette(&mut hooks, &COMMANDS);
    let mut exit = hooks.use_exit();

    hooks.use_event_handler(EventScope::Current, EventPriority::Normal, move |event| {
        let Event::Key(key) = event else {
            return EventResult::Ignored;
        };
        if key.kind != KeyEventKind::Press {
            return EventResult::Ignored;
        }

        if matches!(key.code, KeyCode::Char('q') | KeyCode::Char('Q')) {
            exit();
            EventResult::Consumed
        } else {
            EventResult::Ignored
        }
    });

    element!(
        Center(
            width: Constraint::Length(92),
            height: Constraint::Length(22),
        ) {
            Border(
                flex_direction: Direction::Vertical,
                border_style: Style::new().blue(),
                top_title: Line::from(" custom hook command palette ").blue().bold().centered(),
                bottom_title: Line::from(" type filter | j/k move | Enter submit | Esc clear | q quit ").dark_gray().centered(),
            ) {
                View(
                    flex_direction: Direction::Horizontal,
                    gap: 2,
                ) {
                    Border(
                        width: Constraint::Fill(2),
                        flex_direction: Direction::Vertical,
                        border_style: Style::new().cyan(),
                        top_title: Line::from(format!(" commands / {} match(es) ", palette.visible.len())).cyan().bold().centered(),
                    ) {
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from(format!("filter: {}", palette.query)).centered())
                        }
                        View(height: Constraint::Length(1)) {
                            Text(text: "")
                        }
                        if palette.visible.is_empty() {
                            View(height: Constraint::Length(1)) {
                                Text(text: Line::from("no command matches this filter").centered(), style: Style::new().dark_gray())
                            }
                        } else {
                            for (position, command_index) in palette.visible.iter().copied().enumerate() {
                                CommandRow(
                                    command: COMMANDS[command_index],
                                    active: position == palette.selected_position,
                                    key: COMMANDS[command_index].title,
                                )
                            }
                        }
                    }
                    View(
                        width: Constraint::Fill(1),
                        flex_direction: Direction::Vertical,
                        gap: 1,
                    ) {
                        Border(
                            height: Constraint::Length(8),
                            flex_direction: Direction::Vertical,
                            justify_content: Flex::Center,
                            border_style: Style::new().green(),
                            top_title: Line::from(" hook snapshot ").green().bold().centered(),
                        ) {
                            Text(text: Line::from(format!("query: {}", palette.query)).centered())
                            Text(text: Line::from(format!("cursor: {}", palette.selected_position)).centered())
                            Text(text: Line::from(format!("matches: {}", palette.visible.len())).centered())
                            Text(text: Line::from(selected_title(palette.selected)).centered())
                        }
                        Border(
                            height: Constraint::Fill(1),
                            flex_direction: Direction::Vertical,
                            justify_content: Flex::Center,
                            border_style: Style::new().magenta(),
                            top_title: Line::from(" event ").magenta().bold().centered(),
                        ) {
                            Text(text: Line::from(palette.status).centered(), wrap: true)
                        }
                    }
                }
            }
        }
    )
}

struct CommandPalette {
    query: String,
    visible: Vec<usize>,
    selected_position: usize,
    selected: Option<Command>,
    status: String,
}

fn use_command_palette(hooks: &mut Hooks, commands: &'static [Command]) -> CommandPalette {
    let query = hooks.use_state(String::new);
    let mut cursor = hooks.use_state(|| 0usize);
    let mut status = hooks.use_state(|| "type to filter commands".to_string());

    let query_text = query.read().clone();
    let query_deps = query_text.clone();
    let visible = hooks.use_memo(move || matching_indices(commands, &query_text), query_deps);

    let visible_len = visible.len();
    let cursor_value = cursor.get();
    hooks.use_effect(
        move || {
            let next_cursor = if visible_len == 0 {
                0
            } else {
                cursor_value.min(visible_len - 1)
            };
            if next_cursor != cursor_value {
                cursor.set(next_cursor);
            }
        },
        (visible_len, cursor_value),
    );

    let selected_position = if visible.is_empty() {
        0
    } else {
        cursor.get().min(visible.len() - 1)
    };
    let selected = visible
        .get(selected_position)
        .and_then(|index| commands.get(*index))
        .copied();
    let visible_for_events = visible.clone();

    hooks.use_event_handler(EventScope::Current, EventPriority::High, move |event| {
        let Event::Key(key) = event else {
            return EventResult::Ignored;
        };
        if key.kind != KeyEventKind::Press {
            return EventResult::Ignored;
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') => return EventResult::Ignored,
            KeyCode::Char('j') | KeyCode::Down => {
                move_cursor(cursor, visible_for_events.len(), 1);
                status.set("cursor moved down".to_string());
            }
            KeyCode::Char('k') | KeyCode::Up => {
                move_cursor(cursor, visible_for_events.len(), -1);
                status.set("cursor moved up".to_string());
            }
            KeyCode::Enter => {
                if let Some(command) = selected {
                    status.set(format!("submitted: {}", command.title));
                } else {
                    status.set("nothing to submit".to_string());
                }
            }
            KeyCode::Backspace => {
                query.write().pop();
                cursor.set(0);
                status.set(query_status(query));
            }
            KeyCode::Esc => {
                query.write().clear();
                cursor.set(0);
                status.set("filter cleared".to_string());
            }
            KeyCode::Char(ch) if !ch.is_control() => {
                query.write().push(ch);
                cursor.set(0);
                status.set(query_status(query));
            }
            _ => return EventResult::Ignored,
        }

        EventResult::Consumed
    });

    CommandPalette {
        query: query.read().clone(),
        visible,
        selected_position,
        selected,
        status: status.read().clone(),
    }
}

#[derive(Default, Props)]
struct CommandRowProps {
    command: Command,
    active: bool,
}

#[component]
fn CommandRow(props: &CommandRowProps, _hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let marker = if props.active { ">" } else { " " };
    let style = if props.active {
        Style::new().black().on_cyan()
    } else {
        Style::new()
    };
    let group_style = if props.active {
        Style::new().black().on_cyan()
    } else {
        Style::new().fg(Color::Yellow)
    };

    element!(
        View(height: Constraint::Length(1)) {
            Text(text: Line::from(vec![
                format!("{marker} {:<18}", props.command.title).into(),
                ratatui_kit::ratatui::text::Span::styled(
                    format!(" {:<8}", props.command.group),
                    group_style,
                ),
                format!(" {}", props.command.shortcut).into(),
            ]).style(style))
        }
    )
}

fn matching_indices(commands: &[Command], query: &str) -> Vec<usize> {
    let needle = query.trim().to_lowercase();
    commands
        .iter()
        .enumerate()
        .filter_map(|(index, command)| {
            (needle.is_empty()
                || command.title.to_lowercase().contains(&needle)
                || command.group.to_lowercase().contains(&needle))
            .then_some(index)
        })
        .collect()
}

fn move_cursor(mut cursor: State<usize>, len: usize, offset: isize) {
    if len == 0 {
        cursor.set(0);
        return;
    }

    let next = cursor
        .get()
        .saturating_add_signed(offset)
        .min(len.saturating_sub(1));
    cursor.set(next);
}

fn selected_title(command: Option<Command>) -> String {
    command
        .map(|command| format!("selected: {}", command.title))
        .unwrap_or_else(|| "selected: <none>".to_string())
}

fn query_status(query: State<String>) -> String {
    let query = query.read();
    if query.is_empty() {
        "filter cleared".to_string()
    } else {
        format!("filter changed: {}", query.as_str())
    }
}
