//! Todo App 组合示例。
//!
//! 一个小型工作台：背景层负责列表导航和过滤，`SearchInput` 负责新增任务，
//! `ConfirmModal` 负责删除确认，展示多个内置能力如何组合成真实应用流。

use ratatui_kit::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    ratatui::{
        layout::{Alignment, Constraint, Direction, Flex},
        style::{Color, Style, Stylize},
        text::Line,
    },
};

#[derive(Clone)]
struct Todo {
    title: String,
    done: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TodoFilter {
    All,
    Open,
    Done,
}

impl TodoFilter {
    fn next(self) -> Self {
        match self {
            Self::All => Self::Open,
            Self::Open => Self::Done,
            Self::Done => Self::All,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Open => "open",
            Self::Done => "done",
        }
    }

    fn accepts(self, todo: &Todo) -> bool {
        match self {
            Self::All => true,
            Self::Open => !todo.done,
            Self::Done => todo.done,
        }
    }
}

#[tokio::main]
async fn main() {
    element!(TodoApp)
        .fullscreen()
        .await
        .expect("Failed to run the application");
}

#[component]
fn TodoApp(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let todos = hooks.use_state(initial_todos);
    let mut cursor = hooks.use_state(|| 0usize);
    let mut filter = hooks.use_state(|| TodoFilter::All);
    let mut draft = hooks.use_state(String::new);
    let mut pending_delete = hooks.use_state(|| None::<usize>);
    let mut status = hooks.use_state(|| "ready".to_string());
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
            KeyCode::Char('j') | KeyCode::Down => {
                move_cursor(todos, cursor, filter.get(), 1);
                EventResult::Consumed
            }
            KeyCode::Char('k') | KeyCode::Up => {
                move_cursor(todos, cursor, filter.get(), -1);
                EventResult::Consumed
            }
            KeyCode::Char(' ') | KeyCode::Enter => {
                toggle_current(todos, cursor, status);
                EventResult::Consumed
            }
            KeyCode::Char('f') | KeyCode::Char('F') => {
                let next_filter = filter.get().next();
                filter.set(next_filter);
                let next_cursor = clamp_cursor(&todos.read(), cursor.get(), next_filter);
                cursor.set(next_cursor);
                status.set(format!("filter: {}", next_filter.label()));
                EventResult::Consumed
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                if todos.read().is_empty() {
                    status.set("nothing to delete".to_string());
                } else {
                    pending_delete.set(Some(cursor.get()));
                    status.set("delete confirmation opened".to_string());
                }
                EventResult::Consumed
            }
            _ => EventResult::Ignored,
        }
    });

    let filter_view = filter.get();
    let todos_view = todos.read().clone();
    let visible_indices = visible_indices(&todos_view, filter_view);
    let cursor_view = if visible_indices.contains(&cursor.get()) {
        cursor.get()
    } else {
        visible_indices.first().copied().unwrap_or(0)
    };
    let rows = todo_rows(&todos_view, &visible_indices, cursor_view);
    let open_count = todos_view.iter().filter(|todo| !todo.done).count();
    let done_count = todos_view.len().saturating_sub(open_count);
    let pending_index = pending_delete.get();
    let pending_title = pending_index
        .and_then(|index| todos_view.get(index))
        .map(|todo| todo.title.clone())
        .unwrap_or_else(|| "selected task".to_string());
    let status_view = status.read().to_string();
    let selected_label = selected_label(&todos_view, cursor_view);

    element!(
        Center(
            width: Constraint::Length(100),
            height: Constraint::Length(24),
        ) {
            Border(
                flex_direction: Direction::Vertical,
                gap: 1,
                border_style: Style::new().blue(),
                top_title: Line::from(" todo app ").blue().bold().centered(),
                bottom_title: Line::from(" a add | j/k move | Space toggle | f filter | d delete | q quit ").dark_gray().centered(),
            ) {
                SearchInput(
                    width: Constraint::Fill(1),
                    value: draft.read().to_string(),
                    placeholder: "Press a to add a task".to_string(),
                    activate_key: KeyCode::Char('a'),
                    on_change: move |next: String| draft.set(next),
                    on_submit: move |value: String| {
                        let title = value.trim().to_string();
                        if title.len() < 3 {
                            status.set("task title needs at least 3 chars".to_string());
                            return false;
                        }

                        let mut todo_list = todos.write();
                        todo_list.push(Todo {
                            title: title.clone(),
                            done: false,
                        });
                        let new_index = todo_list.len().saturating_sub(1);
                        drop(todo_list);

                        filter.set(TodoFilter::All);
                        cursor.set(new_index);
                        status.set(format!("added: {title}"));
                        true
                    },
                    validate: move |value: String| {
                        let len = value.trim().chars().count();
                        if len == 0 {
                            (true, "type a task".to_string())
                        } else if len < 3 {
                            (false, "too short".to_string())
                        } else {
                            (true, "Enter adds task".to_string())
                        }
                    },
                    clear_on_submit: true,
                    clear_on_escape: true,
                    border_style: Style::new().cyan(),
                    active_border_style: Style::new().yellow(),
                    success_border_style: Style::new().green(),
                    error_border_style: Style::new().red(),
                    cursor_style: Style::new().bg(Color::Yellow),
                )
                View(
                    flex_direction: Direction::Horizontal,
                    gap: 2,
                ) {
                    Border(
                        width: Constraint::Length(58),
                        flex_direction: Direction::Vertical,
                        border_style: Style::new().cyan(),
                        top_title: Line::from(format!(" tasks / {} ", filter_view.label())).cyan().centered(),
                    ) {
                        if rows.is_empty() {
                            View(
                                height: Constraint::Fill(1),
                                justify_content: Flex::Center,
                            ) {
                                Text(
                                    text: Line::from("No tasks in this filter").yellow(),
                                    alignment: Alignment::Center,
                                )
                            }
                        } else {
                            for (index, row) in rows.into_iter().enumerate() {
                                View(height: Constraint::Length(1), key: index) {
                                    Text(text: row)
                                }
                            }
                        }
                    }
                    View(
                        width: Constraint::Fill(1),
                        flex_direction: Direction::Vertical,
                        gap: 1,
                    ) {
                        Border(
                            height: Constraint::Length(7),
                            flex_direction: Direction::Vertical,
                            justify_content: Flex::Center,
                            border_style: Style::new().cyan(),
                            top_title: Line::from(" state ").cyan().centered(),
                        ) {
                            View(height: Constraint::Length(1)) {
                                Text(text: Line::from(format!("open: {open_count}")).centered())
                            }
                            View(height: Constraint::Length(1)) {
                                Text(text: Line::from(format!("done: {done_count}")).centered())
                            }
                            View(height: Constraint::Length(1)) {
                                Text(text: Line::from(format!("selected: {selected_label}")).centered())
                            }
                        }
                        Border(
                            flex_direction: Direction::Vertical,
                            justify_content: Flex::Center,
                            border_style: Style::new().cyan(),
                            top_title: Line::from(" event ").cyan().centered(),
                        ) {
                            Text(text: Line::from(status_view).centered(), wrap: true)
                        }
                    }
                }
            }
            ConfirmModal(
                open: pending_index.is_some(),
                width: Constraint::Length(72),
                height: Constraint::Length(10),
                title: Line::from("Delete task?"),
                content: format!("Remove {pending_title}?"),
                confirm_text: "Delete".to_string(),
                cancel_text: "Keep".to_string(),
                style: Style::new().dim(),
                border_style: Style::new().yellow(),
                title_style: Style::new().yellow().bold(),
                button_style: Style::new().gray(),
                selected_button_style: Style::new().yellow().bold(),
                on_confirm: move |_: ()| {
                    if let Some(index) = pending_delete.get() {
                        let mut todo_list = todos.write();
                        let removed = if index < todo_list.len() {
                            Some(todo_list.remove(index).title)
                        } else {
                            None
                        };
                        let next_cursor = clamp_cursor(&todo_list, cursor.get(), filter.get());
                        drop(todo_list);

                        cursor.set(next_cursor);
                        status.set(match removed {
                            Some(title) => format!("deleted: {title}"),
                            None => "task already gone".to_string(),
                        });
                    }
                    pending_delete.set(None);
                },
                on_cancel: move |_: ()| {
                    pending_delete.set(None);
                    status.set("delete canceled".to_string());
                },
            )
        }
    )
}

fn initial_todos() -> Vec<Todo> {
    vec![
        Todo {
            title: "Review runtime".to_string(),
            done: false,
        },
        Todo {
            title: "Record todo app".to_string(),
            done: false,
        },
        Todo {
            title: "Write docs".to_string(),
            done: false,
        },
        Todo {
            title: "Check input layers".to_string(),
            done: true,
        },
        Todo {
            title: "Plan next slice".to_string(),
            done: false,
        },
    ]
}

fn visible_indices(todos: &[Todo], filter: TodoFilter) -> Vec<usize> {
    todos
        .iter()
        .enumerate()
        .filter_map(|(index, todo)| filter.accepts(todo).then_some(index))
        .collect()
}

fn clamp_cursor(todos: &[Todo], current: usize, filter: TodoFilter) -> usize {
    let indices = visible_indices(todos, filter);
    if indices.contains(&current) {
        current
    } else {
        indices.first().copied().unwrap_or(0)
    }
}

fn move_cursor(
    todos: State<Vec<Todo>>,
    mut cursor: State<usize>,
    filter: TodoFilter,
    direction: isize,
) {
    let indices = visible_indices(&todos.read(), filter);
    if indices.is_empty() {
        return;
    }

    let current_position = indices
        .iter()
        .position(|index| *index == cursor.get())
        .unwrap_or(0);
    let next_position = current_position
        .saturating_add_signed(direction)
        .min(indices.len().saturating_sub(1));
    cursor.set(indices[next_position]);
}

fn toggle_current(todos: State<Vec<Todo>>, cursor: State<usize>, mut status: State<String>) {
    let index = cursor.get();
    let mut todo_list = todos.write();
    let Some(todo) = todo_list.get_mut(index) else {
        status.set("no task selected".to_string());
        return;
    };

    todo.done = !todo.done;
    let state = if todo.done { "done" } else { "open" };
    status.set(format!("{state}: {}", todo.title));
}

fn todo_rows(todos: &[Todo], visible_indices: &[usize], cursor: usize) -> Vec<Line<'static>> {
    visible_indices
        .iter()
        .filter_map(|index| todos.get(*index).map(|todo| (*index, todo)))
        .map(|(index, todo)| {
            let marker = if index == cursor { ">" } else { " " };
            let checkbox = if todo.done { "[x]" } else { "[ ]" };
            let style = if index == cursor {
                Style::new().black().on_cyan()
            } else if todo.done {
                Style::new().dark_gray()
            } else {
                Style::new()
            };

            Line::styled(format!("{marker} {checkbox} {}", todo.title), style)
        })
        .collect()
}

fn selected_label(todos: &[Todo], cursor: usize) -> String {
    todos
        .get(cursor)
        .map(|todo| todo.title.clone())
        .unwrap_or_else(|| "<none>".to_string())
}
