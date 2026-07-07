//! WrappedText 内置组件示例。
//!
//! 长正文会按指定宽度预先换行，并把真实行数交给 ScrollView。

use ratatui_kit::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    ratatui::{
        layout::{Constraint, Direction},
        style::{Style, Stylize},
        text::Line,
        widgets::Block,
    },
};

const BODY: &str = "第一段文字会按照 wrap_width 预先换行。这样 ScrollView 拿到的不再是一行很长的 Paragraph，而是已经展开成多行的正文高度。\
\n\n在小说阅读、日志查看、帮助文档和说明面板里，这个高度很关键：滚动容器需要知道内容到底有多少行，才能让滚动条、PageDown 和 End 都落在正确位置。\
\n\n普通 Text(wrap: true) 只负责绘制时软换行；WrappedText 会把换行结果变成布局高度。阅读器类应用常见的正文页也是这个思路，只是这里把它沉淀成框架组件。\
\n\n你可以把它看成一个只读正文块：外层决定视口、边框和滚动状态，WrappedText 专心把纯文本变成稳定的行。它不会内置章节、搜索命中、高亮或业务进度，因为这些都应该由上层组合。\
\n\n当文本宽度来自页面布局时，最好显式传入 wrap_width。当前 update 阶段还拿不到最终绘制宽度，显式宽度可以让 ScrollView 在布局前就得到准确的内容高度。\
\n\nThe same component also works for English prose and long operational notes. It keeps the API small: pass plain text, a wrap width, and optional style. The measured line count is then reflected in layout, so scrolling behaves like a reader rather than a clipped paragraph.\
\n\nA useful rule of thumb: use Text when the phrase is short or richly styled, and use WrappedText when plain prose needs to become part of scrollable layout. This keeps small labels lightweight while making long documents predictable.";

#[tokio::main]
async fn main() {
    element!(WrappedTextApp)
        .fullscreen()
        .await
        .expect("Failed to run the application");
}

#[component]
fn WrappedTextApp(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let scroll_state = hooks.use_state(ScrollViewState::default);
    let mut exit = hooks.use_exit();

    hooks.use_event_handler(EventScope::Current, EventPriority::Normal, move |event| {
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
            KeyCode::Char('j')
            | KeyCode::Down
            | KeyCode::Char('k')
            | KeyCode::Up
            | KeyCode::PageDown
            | KeyCode::PageUp
            | KeyCode::Home
            | KeyCode::End => {
                scroll_state.write().handle_event(&event);
                EventResult::Consumed
            }
            _ => EventResult::Ignored,
        }
    });

    let offset = scroll_state.read().offset();

    element!(
        Center(
            width: Constraint::Length(88),
            height: Constraint::Length(22),
        ) {
            Border(
                flex_direction: Direction::Vertical,
                gap: 1,
                border_style: Style::new().blue(),
                top_title: Line::from(" wrapped text ").blue().bold().centered(),
                bottom_title: Line::from(format!(" j/k scroll | PageDown/PageUp | Home/End | q quit    offset y={}", offset.y)).dark_gray().centered(),
            ) {
                ScrollView(
                    flex_direction: Direction::Vertical,
                    state: scroll_state,
                    scrollbars: Scrollbars {
                        vertical_scrollbar_visibility: ScrollbarVisibility::Always,
                        horizontal_scrollbar_visibility: ScrollbarVisibility::Never,
                        ..Default::default()
                    },
                    block: Block::bordered()
                        .title(Line::from(" measured prose ").cyan().centered())
                        .border_style(Style::new().cyan()),
                ) {
                    WrappedText(
                        text: BODY,
                        wrap_width: 72,
                        style: Style::new().white(),
                    )
                }
            }
        }
    )
}
