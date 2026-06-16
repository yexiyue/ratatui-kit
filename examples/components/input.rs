//! Input 内置组件示例。
//!
//! `Input` 只负责渲染 `tui_input::Input` 状态；键盘事件由页面 handler 转发。

use ratatui_kit::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::{tui_input::backend::crossterm::EventHandler, *},
    ratatui::{
        layout::{Constraint, Direction, Flex},
        style::{Color, Style, Stylize},
        text::Line,
    },
};

const EMPTY_VALUE: &str = "<empty>";

#[tokio::main]
async fn main() {
    element!(App)
        .fullscreen()
        .await
        .expect("Failed to run the application");
}

#[component]
fn App(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let input = hooks.use_state(tui_input::Input::default);
    let history = hooks.use_state(Vec::<String>::default);
    let mut status = hooks.use_state(|| "typing is active".to_string());
    let mut exit = hooks.use_exit();

    hooks.use_event_handler(EventScope::Current, EventPriority::Normal, move |event| {
        let Event::Key(key) = &event else {
            return EventResult::Ignored;
        };
        if key.kind != KeyEventKind::Press {
            return EventResult::Ignored;
        }

        match key.code {
            KeyCode::Esc => {
                if input.read().value().is_empty() {
                    exit();
                } else {
                    input.write().reset();
                    status.set("cleared draft".to_string());
                }
                EventResult::Consumed
            }
            KeyCode::Enter => {
                let submitted = input.read().value().trim().to_string();
                if submitted.is_empty() {
                    status.set("nothing to submit".to_string());
                } else {
                    {
                        let mut items = history.write();
                        items.insert(0, submitted.clone());
                        items.truncate(4);
                    }
                    input.write().reset();
                    status.set(format!("submitted {submitted}"));
                }
                EventResult::Consumed
            }
            _ => {
                input.write().handle_event(&event);
                status.set("editing draft".to_string());
                EventResult::Consumed
            }
        }
    });

    let input_view = input.read().clone();
    let value = input_view.value().to_string();
    let value_label = if value.is_empty() {
        EMPTY_VALUE.to_string()
    } else {
        value
    };
    let cursor = input_view.visual_cursor();
    let history_view = history.read().clone();
    let status_view = status.read().to_string();

    element!(
        Center(
            width: Constraint::Length(92),
            height: Constraint::Length(20),
        ) {
            Border(
                flex_direction: Direction::Vertical,
                gap: 1,
                border_style: Style::new().blue(),
                top_title: Line::from(" input component ").blue().bold().centered(),
                bottom_title: Line::from(" type text | Enter submit | Esc clear / quit ").dark_gray().centered(),
            ) {
                View(
                    flex_direction: Direction::Horizontal,
                    gap: 2,
                ) {
                    View(
                        width: Constraint::Length(56),
                        flex_direction: Direction::Vertical,
                        gap: 1,
                    ) {
                        Border(
                            height: Constraint::Length(3),
                            border_style: Style::new().cyan(),
                            top_title: Line::from(" controlled tui_input::Input ").cyan().centered(),
                        ) {
                            Input(
                                input: input_view,
                                cursor_style: Style::new().bg(Color::Yellow),
                                placeholder: "Type a note and press Enter".to_string(),
                                placeholder_style: Style::new().dark_gray(),
                                style: Style::new().white(),
                                hide_cursor: false,
                            )
                        }
                        Border(
                            height: Constraint::Length(10),
                            flex_direction: Direction::Vertical,
                            border_style: Style::new().cyan(),
                            top_title: Line::from(" submitted ").cyan().centered(),
                        ) {
                            if history_view.is_empty() {
                                View(height: Constraint::Length(1)) {
                                    Text(text: Line::from("No submitted notes yet").dark_gray().centered())
                                }
                            } else {
                                for (index, item) in history_view.iter().enumerate() {
                                    View(
                                        key: index,
                                        height: Constraint::Length(1),
                                    ) {
                                        Text(text: Line::from(format!("{}  {}", index + 1, item)))
                                    }
                                }
                            }
                        }
                    }
                    Border(
                        width: Constraint::Length(28),
                        flex_direction: Direction::Vertical,
                        justify_content: Flex::Center,
                        border_style: Style::new().cyan(),
                        top_title: Line::from(" state ").cyan().centered(),
                    ) {
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from("mode: controlled").centered())
                        }
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from(format!("value: {value_label}")).centered())
                        }
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from(format!("cursor: {cursor}")).centered())
                        }
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from(format!("saved: {}", history_view.len())).centered())
                        }
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from(status_view).centered())
                        }
                    }
                }
            }
        }
    )
}
