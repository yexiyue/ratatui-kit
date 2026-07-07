//! ScrollView 内置组件示例。
//!
//! 按 `j/k` 移动选中行(视口经 `scroll_to_index` 自动跟随),`PageUp/PageDown` 手动浏览,
//! `Home/End` 选首/末行,`b` 切换滚动条盖不盖边框。

use ratatui_kit::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    ratatui::{
        layout::{Constraint, Direction, Flex},
        style::{Color, Modifier, Style, Stylize},
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

    fn line(self, selected: bool) -> Line<'static> {
        let base = match self.kind {
            RowKind::Heading => Line::styled(self.text, Style::new().cyan().bold()),
            RowKind::Text => Line::from(self.text),
            RowKind::Bullet => Line::styled(self.text, Style::new().yellow()),
            RowKind::Code => Line::styled(self.text, Style::new().green()),
        };
        if selected {
            // 选中行加底色高亮 + 前置 ▶,让"选中项联动滚动"一眼可见。
            Line::from(format!("▶ {}", self.text)).style(
                Style::new()
                    .bg(Color::Rgb(45, 55, 95))
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            base
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
    DocRow::new(RowKind::Bullet, "- j / Down select the next row"),
    DocRow::new(RowKind::Bullet, "- k / Up select the previous row"),
    DocRow::new(RowKind::Bullet, "- the viewport follows the selected row"),
    DocRow::new(RowKind::Bullet, "- PageDown / PageUp browse (cursor stays)"),
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
    let mut selected = hooks.use_state(|| 0usize);
    let mut exit = hooks.use_exit();

    let last = DOC_ROWS.len() - 1;

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
                // j/k 移动选中光标,并用 scroll_to_index 让视口自动跟随选中行。
                KeyCode::Char('j') | KeyCode::Down => {
                    let next = (selected.get() + 1).min(last);
                    selected.set(next);
                    scroll_state.write().scroll_to_index(next);
                    status.set(format!("selected row {next}"));
                    EventResult::Consumed
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    let next = selected.get().saturating_sub(1);
                    selected.set(next);
                    scroll_state.write().scroll_to_index(next);
                    status.set(format!("selected row {next}"));
                    EventResult::Consumed
                }
                KeyCode::Home => {
                    selected.set(0);
                    scroll_state.write().scroll_to_index(0);
                    status.set("selected first row".to_string());
                    EventResult::Consumed
                }
                KeyCode::End => {
                    selected.set(last);
                    scroll_state.write().scroll_to_index(last);
                    status.set("selected last row".to_string());
                    EventResult::Consumed
                }
                // PageDown/PageUp 手动滚动浏览(不移动光标)。
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
                _ => EventResult::Ignored,
            }
        },
    );

    let offset = scroll_state.read().offset();
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
                bottom_title: Line::from(" j/k select (view follows) | PageUp/Down browse | Home/End | b border | q quit ").dark_gray().centered(),
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
                                Text(text: row.line(index == selected.get()))
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
                            Text(text: Line::from(format!("selected: {}", selected.get())).centered())
                        }
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from(format!("offset: {}", offset.y)).centered())
                        }
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from(format!("rows: {}", DOC_ROWS.len())).centered())
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
