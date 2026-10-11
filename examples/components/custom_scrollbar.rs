//! `use_scrollbar` hook 示例:内容完全自绘(不是 `ScrollView`),滚动条由 hook
//! 渲染并自带拖拽 —— 抓住滑块拖动 / 点滑轨空白跳转,滑轨两端精确。

use ratatui_kit::{
    RunOptions,
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    ratatui::{
        layout::{Constraint, Direction},
        style::{Style, Stylize},
        text::Line,
    },
};

const TOTAL_LINES: usize = 500;
const PANE_HEIGHT: u16 = 20;

fn line_text(index: usize) -> String {
    match index % 5 {
        0 => format!("{index:>4} │ Custom-rendered content, not a ScrollView."),
        1 => format!("{index:>4} │ The scrollbar comes from the use_scrollbar hook:"),
        2 => format!("{index:>4} │ drag the thumb, or click the track to jump."),
        3 => format!("{index:>4} │ Both ends map exactly to 0 / max offset."),
        _ => format!("{index:>4} │ j/k scroll · PgUp/PgDn page · q quit."),
    }
}

#[tokio::main]
async fn main() {
    element!(App)
        .fullscreen_with(RunOptions { mouse: true })
        .await
        .expect("Failed to run the application");
}

#[component]
fn App(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut offset = hooks.use_state(|| 0usize);
    let mut exit = hooks.use_exit();

    // 视口 = pane 高度 - 上下边框;第一帧尺寸未知时取 0(hook 内部视作不可滚动)。
    let size = hooks.use_previous_size();
    let viewport = size.height.saturating_sub(2) as usize;
    let max_offset = TOTAL_LINES.saturating_sub(viewport);

    hooks.use_event_handler(EventScope::Current, EventPriority::Normal, move |event| {
        let Event::Key(key) = event else {
            return EventResult::Ignored;
        };
        if key.kind != KeyEventKind::Press {
            return EventResult::Ignored;
        }
        let next = match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                exit();
                return EventResult::Consumed;
            }
            KeyCode::Char('j') | KeyCode::Down => offset.get().saturating_add(1),
            KeyCode::Char('k') | KeyCode::Up => offset.get().saturating_sub(1),
            KeyCode::PageDown => offset.get().saturating_add(viewport.max(1)),
            KeyCode::PageUp => offset.get().saturating_sub(viewport.max(1)),
            KeyCode::Home => 0,
            KeyCode::End => max_offset,
            _ => return EventResult::Ignored,
        };
        offset.set(next.min(max_offset));
        EventResult::Consumed
    });

    let current = offset.get().min(max_offset);
    let visible: Vec<Line<'static>> = (current..(current + viewport).min(TOTAL_LINES))
        .map(|index| Line::from(line_text(index)))
        .collect();

    element!(Center(
        width: Constraint::Length(60),
        height: Constraint::Length(PANE_HEIGHT + 2),
    ) {
        ReaderPane(
            lines: visible,
            total: TOTAL_LINES,
            offset: current,
            viewport: viewport,
            max_offset: max_offset,
            on_seek: {
                let mut offset = offset;
                move |target: usize| offset.set(target.min(max_offset))
            },
        )
    })
}

/// 自绘阅读面板:行内容自己渲染,边缘滚动条整条交给 `use_scrollbar` hook
/// (渲染 + 点击/拖拽)。hook 挂在本组件上,滑轨相对本组件 area 落位。
#[component]
fn ReaderPane(props: &mut ReaderPaneProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut on_seek = props.on_seek.take();
    hooks.use_scrollbar(
        ScrollbarOptions {
            content_length: props.total,
            viewport_length: props.viewport,
            position: props.offset,
            ..Default::default()
        },
        move |target: usize| on_seek(target),
    );

    element!(Border(
        flex_direction: Direction::Vertical,
        top_title: Line::from(" custom scrollbar hook ").blue().bold().centered(),
        bottom_title: Line::from(
            format!(" {} / {} ", props.offset, props.max_offset),
        ).dark_gray().centered(),
        border_style: Style::new().blue(),
    ) {
        for line in props.lines.clone() {
            View(height: Constraint::Length(1)) {
                Text(text: line, style: Style::new().white())
            }
        }
    })
}

#[derive(Default, Props)]
struct ReaderPaneProps {
    lines: Vec<Line<'static>>,
    total: usize,
    offset: usize,
    viewport: usize,
    max_offset: usize,
    on_seek: Handler<'static, usize>,
}
