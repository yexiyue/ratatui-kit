//! VirtualList 组合多选状态的示例。

use std::collections::HashSet;

use ratatui_kit::{
    components::tui_widget_list::{ListBuildContext, ListState},
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    ratatui::{
        layout::{Constraint, Direction},
        style::{Color, Style},
        text::Line,
        widgets::Block,
    },
};

#[tokio::main]
async fn main() {
    element!(App)
        .fullscreen()
        .await
        .expect("Failed to run the application");
}

#[component]
fn App(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut loaded = hooks.use_state(|| false);
    let list_state = hooks.use_state(ListState::default);
    let selected = hooks.use_state(HashSet::<usize>::default);
    let mut message = hooks
        .use_state(|| "Loading rows · j/k move · Space toggle · Enter submit · q quit".to_string());
    let mut exit = hooks.use_exit();

    hooks.use_future(async move {
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
        loaded.set(true);
        message.set("Loaded · default cursor restored to row 6".to_string());
    });

    hooks.use_event_handler(EventScope::Current, EventPriority::Normal, move |event| {
        let Event::Key(key) = event else {
            return EventResult::Ignored;
        };
        if key.kind != KeyEventKind::Press {
            return EventResult::Ignored;
        }

        match key.code {
            KeyCode::Char(' ') if loaded.get() => {
                if let Some(index) = list_state.read().selected {
                    let mut selected_items = selected.write();
                    if !selected_items.insert(index) {
                        selected_items.remove(&index);
                    }
                    let count = selected_items.len();
                    message.set(format!("Selected {count} row(s)"));
                }
                EventResult::Consumed
            }
            KeyCode::Enter if loaded.get() => {
                let count = selected.read().len();
                message.set(format!("Submitted {count} selected row(s)"));
                EventResult::Consumed
            }
            KeyCode::Char('q') => {
                exit();
                EventResult::Consumed
            }
            _ => EventResult::Ignored,
        }
    });

    let item_count = if loaded.get() { 10_000usize } else { 0usize };
    let selected_snapshot = selected.read().clone();

    element!(
        View(
            flex_direction: Direction::Vertical,
            gap: 1,
        ) {
            VirtualList<Line<'static>>(
                state: list_state,
                item_count: item_count,
                default_index: Some(5usize),
                block: Block::bordered()
                    .border_style(Style::default().fg(Color::Blue))
                    .title_top(Line::from("Virtual Multi Select").centered())
                    .title_bottom(Line::from("Space toggles · Enter submits").centered()),
                scroll_padding: 2u16,
                infinite_scrolling: false,
                render_item: move |context: &ListBuildContext| {
                    let checked = if selected_snapshot.contains(&context.index) {
                        "[x]"
                    } else {
                        "[ ]"
                    };
                    let label = format!("{checked} Row {:05}", context.index + 1);
                    let style = if context.is_selected {
                        Style::default().black().on_cyan()
                    } else if selected_snapshot.contains(&context.index) {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    };

                    (Line::styled(label, style), 1u16)
                },
            )
            View(height: Constraint::Length(1)) {
                Text(text: message.read().to_string())
            }
        }
    )
}
