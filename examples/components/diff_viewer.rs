//! Diff 内置组件示例。

use ratatui_kit::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    ratatui::{
        layout::{Constraint, Direction},
        style::{Style, Stylize},
        text::Line,
    },
};

struct Sample {
    label: &'static str,
    old: &'static str,
    new: &'static str,
}

const SAMPLES: &[Sample] = &[
    Sample {
        label: "single-line change",
        old: "fn main() {\n    let x = 1;\n    let y = 2;\n    println!(\"sum = {}\", x + y);\n}",
        new: "fn main() {\n    let x = 1;\n    let y = 3;\n    println!(\"sum = {}\", x + y);\n}",
    },
    Sample {
        label: "add line",
        old: "line 1\nline 2\nline 3",
        new: "line 1\nline 2\nline 2.5\nline 3",
    },
    Sample {
        label: "remove line",
        old: "line 1\nline 2\nline 3",
        new: "line 1\nline 3",
    },
    Sample {
        label: "CJK mixed",
        old: "这是第一行\n这是要删除的行\n中英文 mixed content",
        new: "这是第一行\n这是新增的行\n中英文 mixed content",
    },
    Sample {
        label: "empty → content",
        old: "",
        new: "fn hello() {\n    return 42;\n}",
    },
    Sample {
        label: "content → empty",
        old: "fn hello() {\n    return 42;\n}",
        new: "",
    },
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
    let mut show_numbers = hooks.use_state(|| false);
    let mut idx = hooks.use_state(|| 0usize);
    let mut exit = hooks.use_exit();

    hooks.use_event_handler(EventScope::Current, EventPriority::Normal, move |event| {
        if let Event::Key(key) = event
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Char('n') => {
                    show_numbers.set(!show_numbers.get());
                    return EventResult::Consumed;
                }
                KeyCode::Right => {
                    idx.set((idx.get() + 1) % SAMPLES.len());
                    return EventResult::Consumed;
                }
                KeyCode::Left => {
                    idx.set(if idx.get() == 0 {
                        SAMPLES.len() - 1
                    } else {
                        idx.get() - 1
                    });
                    return EventResult::Consumed;
                }
                KeyCode::Char('q') => {
                    exit();
                    return EventResult::Consumed;
                }
                _ => {}
            }
        }
        EventResult::Ignored
    });

    let i = idx.get();
    let sample = &SAMPLES[i];
    let numbers = if show_numbers.get() { "on" } else { "off" };

    element!(
        ScrollView(
            flex_direction: Direction::Vertical,
            scroll_bars: ScrollBars {
                vertical_scrollbar_visibility: ScrollbarVisibility::Automatic,
                ..Default::default()
            },
        ) {
            Border(
                flex_direction: Direction::Vertical,
                gap: 1,
                border_style: Style::new().blue(),
                top_title: Line::from(" diff component ").blue().bold().centered(),
                bottom_title: Line::from(" <- -> sample | n numbers | q quit ").dark_gray().centered(),
            ) {
                View(height: Constraint::Length(1)) {
                    Text(text: Line::from(format!(
                        "sample {}/{}: {}  |  numbers: {}",
                        i + 1,
                        SAMPLES.len(),
                        sample.label,
                        numbers,
                    )))
                }
                Diff(old: sample.old, new: sample.new, show_line_numbers: show_numbers.get())
            }
        }
    )
}
