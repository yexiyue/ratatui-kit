//! ScrollView 内置组件示例。
//!
//! 按 `j/k` 或方向键逐行滚动，按 `PageUp/PageDown` 翻页，按 `Home/End` 跳转。

use ratatui_kit::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    ratatui::{
        layout::{Constraint, Direction, Flex},
        style::{Style, Stylize},
        text::Line,
        widgets::Block,
    },
};

#[derive(Clone, Copy)]
enum RowKind {
    Heading,
    Text,
    Bullet,
    Code,
}

#[derive(Clone, Copy)]
struct DocRow {
    kind: RowKind,
    text: &'static str,
}

impl DocRow {
    const fn new(kind: RowKind, text: &'static str) -> Self {
        Self { kind, text }
    }

    fn line(self) -> Line<'static> {
        match self.kind {
            RowKind::Heading => Line::styled(self.text, Style::new().cyan().bold()),
            RowKind::Text => Line::from(self.text),
            RowKind::Bullet => Line::styled(self.text, Style::new().yellow()),
            RowKind::Code => Line::styled(self.text, Style::new().green()),
        }
    }
}

const DOC_ROWS: [DocRow; 32] = [
    DocRow::new(RowKind::Heading, "ScrollView playbook"),
    DocRow::new(
        RowKind::Text,
        "Use it when content is taller than its viewport.",
    ),
    DocRow::new(
        RowKind::Text,
        "The child tree is normal ratatui-kit layout.",
    ),
    DocRow::new(RowKind::Bullet, "- j / Down scroll one row down"),
    DocRow::new(RowKind::Bullet, "- k / Up scroll one row up"),
    DocRow::new(
        RowKind::Bullet,
        "- PageDown and PageUp move by the viewport",
    ),
    DocRow::new(RowKind::Bullet, "- Home and End jump to the edges"),
    DocRow::new(RowKind::Heading, "Automatic state"),
    DocRow::new(RowKind::Text, "Omit state for the built-in handler."),
    DocRow::new(RowKind::Code, "ScrollView { /* rows */ }"),
    DocRow::new(
        RowKind::Text,
        "The handler lives on the current input layer.",
    ),
    DocRow::new(RowKind::Text, "It consumes only the keys it scrolls on."),
    DocRow::new(RowKind::Heading, "Controlled state"),
    DocRow::new(
        RowKind::Text,
        "Pass a State<ScrollViewState> to own the offset.",
    ),
    DocRow::new(RowKind::Code, "let scroll = hooks.use_state(...);"),
    DocRow::new(RowKind::Code, "ScrollView(state: scroll) { ... }"),
    DocRow::new(RowKind::Text, "The page can inspect offset and jump."),
    DocRow::new(RowKind::Heading, "Layout contract"),
    DocRow::new(RowKind::Text, "Each child contributes width and height."),
    DocRow::new(
        RowKind::Text,
        "The content buffer can grow beyond viewport.",
    ),
    DocRow::new(RowKind::Text, "Scrollbars reserve space before layout."),
    DocRow::new(RowKind::Heading, "Inside modals"),
    DocRow::new(RowKind::Text, "Works inside Modal and ShortcutInfoModal."),
    DocRow::new(RowKind::Text, "Modal layer blocks lower handlers."),
    DocRow::new(RowKind::Heading, "Practical notes"),
    DocRow::new(RowKind::Text, "Use fixed row heights for stable distance."),
    DocRow::new(RowKind::Text, "Use Block when viewport needs a border."),
    DocRow::new(
        RowKind::Text,
        "Use Never visibility to hide an unused scrollbar.",
    ),
    DocRow::new(RowKind::Text, "Use Always when layout must stay stable."),
    DocRow::new(RowKind::Heading, "End of document"),
    DocRow::new(
        RowKind::Text,
        "The status panel shows the controlled offset.",
    ),
    DocRow::new(RowKind::Text, "Press q to leave the example."),
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
    let scroll_state = hooks.use_state(ScrollViewState::default);
    let mut status = hooks.use_state(|| "ready at top".to_string());
    let mut over_border = hooks.use_state(|| true);
    let mut exit = hooks.use_exit();

    hooks.use_event_handler_with_options(
        EventScope::Current,
        EventPriority::Normal,
        EventOptions { hit_test: true },
        move |event| {
            let Event::Key(key) = &event else {
                scroll_state.write().handle_event(&event);
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
                KeyCode::Char('b') | KeyCode::Char('B') => {
                    let next = !over_border.get();
                    over_border.set(next);
                    status.set(if next {
                        "scrollbar on the border".to_string()
                    } else {
                        "scrollbar inset in the border".to_string()
                    });
                    EventResult::Consumed
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    scroll_state.write().handle_event(&event);
                    status.set("scroll down one row".to_string());
                    EventResult::Consumed
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    scroll_state.write().handle_event(&event);
                    status.set("scroll up one row".to_string());
                    EventResult::Consumed
                }
                KeyCode::PageDown => {
                    scroll_state.write().handle_event(&event);
                    status.set("page down".to_string());
                    EventResult::Consumed
                }
                KeyCode::PageUp => {
                    scroll_state.write().handle_event(&event);
                    status.set("page up".to_string());
                    EventResult::Consumed
                }
                KeyCode::Home => {
                    scroll_state.write().handle_event(&event);
                    status.set("jumped to top".to_string());
                    EventResult::Consumed
                }
                KeyCode::End => {
                    scroll_state.write().handle_event(&event);
                    status.set("jumped to bottom".to_string());
                    EventResult::Consumed
                }
                _ => EventResult::Ignored,
            }
        },
    );

    let offset = scroll_state.get().offset();
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
                top_title: Line::from(" scroll view ").blue().bold().centered(),
                bottom_title: Line::from(" j/k scroll | PageDown/PageUp page | Home/End jump | b border | q quit ").dark_gray().centered(),
            ) {
                View(
                    flex_direction: Direction::Horizontal,
                    gap: 2,
                ) {
                    ScrollView(
                        width: Constraint::Length(66),
                        flex_direction: Direction::Vertical,
                        state: scroll_state,
                        scrollbars: Scrollbars {
                            vertical_scrollbar_visibility: ScrollbarVisibility::Always,
                            horizontal_scrollbar_visibility: ScrollbarVisibility::Never,
                            over_border: over_border.get(),
                            ..Default::default()
                        },
                        block: Block::bordered()
                            .title(Line::from(" controlled document ").cyan().centered())
                            .border_style(Style::new().cyan()),
                    ) {
                        for (index, row) in DOC_ROWS.into_iter().enumerate() {
                            View(
                                key: index,
                                height: Constraint::Length(1),
                            ) {
                                Text(text: row.line())
                            }
                        }
                    }
                    Border(
                        width: Constraint::Length(22),
                        flex_direction: Direction::Vertical,
                        justify_content: Flex::Center,
                        border_style: Style::new().cyan(),
                        top_title: Line::from(" state ").cyan().centered(),
                    ) {
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from(format!("offset: {}", offset.y)).centered())
                        }
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from(format!("rows: {}", DOC_ROWS.len())).centered())
                        }
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from(status_view).centered())
                        }
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from("controlled").centered())
                        }
                    }
                }
            }
        }
    )
}
