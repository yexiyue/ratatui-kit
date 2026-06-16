//! VirtualList 内置组件示例。

use ratatui_kit::{
    components::tui_widget_list::{ListBuildContext, ListState},
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    ratatui::{
        layout::{Constraint, Direction, Flex},
        style::{Color, Style, Stylize},
        text::Line,
        widgets::Block,
    },
};

const ROW_COUNT: usize = 10_000;
const DEFAULT_INDEX: usize = 42;

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
    let mut empty = hooks.use_state(|| false);
    let list_state = hooks.use_state(ListState::default);
    let mut submitted = hooks.use_state(|| "not submitted".to_string());
    let mut exit = hooks.use_exit();

    hooks.use_future(async move {
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
        loaded.set(true);
        submitted.set("loaded rows".to_string());
    });

    hooks.use_event_handler(EventScope::Current, EventPriority::Normal, move |event| {
        if let Event::Key(key) = event
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Char('e') => {
                    let was_empty = empty.get();
                    empty.set(!was_empty);
                    submitted.set(if was_empty {
                        "rows restored".to_string()
                    } else {
                        "empty list".to_string()
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

    let item_count = if loaded.get() && !empty.get() {
        ROW_COUNT
    } else {
        0
    };
    let mode = if !loaded.get() {
        "loading"
    } else if empty.get() {
        "empty"
    } else {
        "ready"
    };
    let cursor = cursor_label(list_state.read().selected);
    let submitted_view = submitted.read().to_string();
    let row_count_label = if item_count == 0 {
        "<none>".to_string()
    } else {
        format!("{item_count} rows")
    };
    let list_hint = if !loaded.get() {
        " loading rows "
    } else if empty.get() {
        " empty list "
    } else {
        " visible rows only "
    };

    element!(
        Center(
            width: Constraint::Length(90),
            height: Constraint::Length(20),
        ) {
            Border(
                flex_direction: Direction::Vertical,
                border_style: Style::new().fg(Color::Blue),
                top_title: Line::from(" virtual list ").fg(Color::Blue).bold().centered(),
                bottom_title: Line::from(" j/k move | Home/End jump | Enter select | e empty | q quit ").dark_gray().centered(),
            ) {
                View(
                    flex_direction: Direction::Horizontal,
                    gap: 2,
                ) {
                    VirtualList<Line<'static>>(
                        width: Constraint::Length(42),
                        state: list_state,
                        item_count: item_count,
                        default_index: Some(DEFAULT_INDEX),
                        block: Block::bordered()
                            .border_style(Style::new().fg(Color::Cyan))
                            .title_top(Line::from(" build log ").centered())
                            .title_bottom(Line::from(list_hint).centered()),
                        scroll_padding: 2u16,
                        infinite_scrolling: false,
                        render_item: |context: &ListBuildContext| {
                            let row = context.index + 1;
                            let label = format!("row {row:05}  task/{:03}", row % 128);
                            let style = if context.is_selected {
                                Style::new().fg(Color::Black).bg(Color::Green)
                            } else if row.is_multiple_of(25) {
                                Style::new().fg(Color::Yellow)
                            } else {
                                Style::new()
                            };

                            (Line::styled(label, style), 1u16)
                        },
                        on_select: move |index: usize| {
                            submitted.set(format!("selected row {}", index + 1));
                        },
                    )
                    Border(
                        width: Constraint::Fill(1),
                        flex_direction: Direction::Vertical,
                        justify_content: Flex::Center,
                        border_style: Style::new().fg(Color::Cyan),
                        top_title: Line::from(" state ").fg(Color::Cyan).centered(),
                    ) {
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from(format!("mode: {mode}")).centered())
                        }
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from(format!("rows: {row_count_label}")).centered())
                        }
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from(format!("cursor: {cursor}")).centered())
                        }
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from(format!("submit: {submitted_view}")).centered())
                        }
                    }
                }
            }
        }
    )
}

fn cursor_label(index: Option<usize>) -> String {
    index.map_or_else(|| "<none>".to_string(), |index| format!("#{}", index + 1))
}
