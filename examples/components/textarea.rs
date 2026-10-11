//! TextArea 内置组件示例。
//!
//! 上半区:单行模式,Enter 提交并写入历史;下半区:多行模式,展示软换行、
//! 显示行 Up/Down 导航与鼠标点击定位光标。

use ratatui_kit::{
    RunOptions,
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    ratatui::{
        layout::{Constraint, Direction},
        style::Style,
        text::Line,
    },
};

#[tokio::main]
async fn main() {
    element!(App)
        .fullscreen_with(RunOptions { mouse: true })
        .await
        .expect("Failed to run the application");
}

#[component]
fn App(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let value = hooks.use_state(String::new);
    let notes = hooks.use_state(|| "第一行:支持中英文混排与软换行".to_string());
    let history = hooks.use_state(Vec::<String>::default);
    let mut exit = hooks.use_exit();

    hooks.use_event_handler(EventScope::Current, EventPriority::Normal, move |event| {
        if let Event::Key(key) = event
            && key.kind == KeyEventKind::Press
            && key.code == KeyCode::Esc
        {
            exit();
            EventResult::Consumed
        } else {
            EventResult::Ignored
        }
    });

    let history_snapshot = history.read().clone();

    element!(Center(
        width: Constraint::Percentage(70),
        height: Constraint::Percentage(80),
    ) {
        Border(
            flex_direction: Direction::Vertical,
            gap: 1,
        ) {
            Border(
                height: Constraint::Length(8),
                top_title: Line::from("单行输入(Enter 提交)"),
                bottom_title: Line::from("Esc 退出 · 点击输入框定位光标"),
            ) {
                View {
                    TextArea(
                        value: value.read().to_string(),
                        is_focus: true,
                        multiline: false,
                        placeholder: "输入后按 Enter …",
                        on_change: {
                            let mut value = value;
                            move |next: String| value.set(next)
                        },
                        on_submit: {
                            let mut value = value;
                            move |text: String| {
                                history.write().push(text);
                                value.set(String::new());
                            }
                        },
                    )
                    Text(
                        text: TextParagraph::from(if history_snapshot.is_empty() {
                            "<尚无提交记录>".to_string()
                        } else {
                            format!("最近提交: {}", history_snapshot.last().unwrap())
                        }),
                        style: Style::default().dim(),
                    )
                }
            }

            Border(top_title: Line::from("多行笔记(软换行 + 点击定位)")) {
                TextArea(
                    value: notes.read().to_string(),
                    is_focus: false,
                    multiline: true,
                    placeholder: "失焦演示:仍可点击定位…",
                    on_change: {
                        let mut notes = notes;
                        move |next: String| notes.set(next)
                    },
                )
            }
        }
    })
}
