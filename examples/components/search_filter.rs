//! SearchInput + 客户端过滤 + 方向键导航示例。
//!
//! 在搜索框输入 "bob"，列表只保留名字里包含 "bob" 的结果；编辑态内 `↑/↓` 在命中项间移动，
//! `Enter` 确认选中的那一项（无需把名字打完整）。方向键经 `SearchInput.on_arrow` 回调消费，
//! 不会写进输入框；`.filter()` + `.contains()` 的过滤逻辑写在业务代码里。

use ratatui_kit::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    ratatui::{
        layout::{Constraint, Direction},
        style::{Color, Style, Stylize},
        text::Line,
    },
};

const PEOPLE: [&str; 9] = [
    "Alice Bobson",
    "Bob Marley",
    "Bobby Fischer",
    "Carol Bob",
    "Bob The Builder",
    "Carla Programming",
    "Debra Casey",
    "Eddie Thomas",
    "Fiona Smith",
];

fn filtered(query: &str) -> Vec<&'static str> {
    let q = query.to_ascii_lowercase();
    PEOPLE
        .iter()
        .copied()
        .filter(|name| q.is_empty() || name.to_ascii_lowercase().contains(&q))
        .collect()
}

#[tokio::main]
async fn main() {
    element!(App)
        .fullscreen()
        .await
        .expect("Failed to run the application");
}

#[component]
fn App(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut query = hooks.use_state(String::new);
    let mut selected = hooks.use_state(|| 0usize);
    let mut chosen = hooks.use_state(|| None::<String>);
    let mut exit = hooks.use_exit();

    // 核心：过滤只保留包含 query 的结果；query 为空时显示全部。
    let current_query = query.read().to_ascii_lowercase();
    let visible = filtered(&current_query);
    let visible_len = visible.len();

    // 过滤结果变少时，把选中项夹回合法范围。
    hooks.use_effect(
        move || {
            let last = visible_len.saturating_sub(1);
            if selected.get() > last {
                selected.set(last);
            }
        },
        visible_len,
    );

    hooks.use_event_handler(EventScope::Current, EventPriority::Normal, move |event| {
        // SearchInput 打开输入层后本 handler 会被截断，因此下述键只在搜索框未编辑时生效。
        if let Event::Key(key) = event
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Char('j') | KeyCode::Down if selected.get() + 1 < visible_len => {
                    selected += 1;
                    return EventResult::Consumed;
                }
                KeyCode::Char('k') | KeyCode::Up if selected.get() > 0 => {
                    selected -= 1;
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

    let selected_index = selected.get();
    let selected_name = visible.get(selected_index).copied().unwrap_or_default();
    let list_lines: Vec<Line<'static>> = visible
        .iter()
        .enumerate()
        .map(|(index, name)| {
            if index == selected_index {
                Line::styled(format!("> {name}"), Style::new().black().on_cyan())
            } else {
                Line::from(format!("  {name}"))
            }
        })
        .collect();
    let chosen_view = chosen.read().clone().unwrap_or_else(|| "none".to_string());

    // 高度随过滤结果动态收缩，让「结果减少」在布局上可见。
    let height = Constraint::Length(8 + (visible_len.max(1)) as u16);

    element!(
        Center(
            width: Constraint::Length(92),
            height: height,
        ) {
            Border(
                flex_direction: Direction::Vertical,
                gap: 1,
                border_style: Style::new().fg(Color::Blue),
                top_title: Line::from(" search + filter + arrows ").fg(Color::Blue).bold().centered(),
                bottom_title: Line::from(" s search | ↑/↓ move | Enter pick | Esc reset | q quit ").dark_gray().centered(),
            ) {
                SearchInput(
                    width: Constraint::Fill(1),
                    value: query.read().to_string(),
                    placeholder: "Press s, then type bob to filter".to_string(),
                    on_change: move |next: String| query.set(next),
                    on_arrow: move |key_code: KeyCode| match key_code {
                        // 编辑态内 ↑/↓ 只移动选中项，不写进输入框。
                        KeyCode::Down if selected.get() + 1 < visible_len => {
                            selected += 1;
                            true
                        }
                        KeyCode::Up if selected.get() > 0 => {
                            selected -= 1;
                            true
                        }
                        // 其它导航键(如 Left/Right)交回输入框移动光标。
                        _ => false,
                    },
                    on_submit: move |value: String| {
                        let list = filtered(&value);
                        match list.get(selected.get()) {
                            Some(&name) => chosen.set(Some(name.to_string())),
                            None => chosen.set(None),
                        }
                        true
                    },
                    clear_on_escape: true,
                    border_style: Style::new().fg(Color::Cyan),
                    active_border_style: Style::new().fg(Color::Yellow),
                    success_border_style: Style::new().fg(Color::Green),
                    error_border_style: Style::new().fg(Color::Red),
                    cursor_style: Style::new().bg(Color::Yellow),
                    validate: move |value: String| {
                        let n = filtered(&value).len();
                        if n == 0 {
                            (false, "no match".to_string())
                        } else {
                            (true, format!("{n} matches · ↑/↓ pick · Enter confirm"))
                        }
                    },
                )
                View(
                    flex_direction: Direction::Horizontal,
                    gap: 2,
                ) {
                    Border(
                        width: Constraint::Length(42),
                        flex_direction: Direction::Vertical,
                        border_style: Style::new().fg(Color::Cyan),
                        top_title: Line::from(" filtered people ").fg(Color::Cyan).centered(),
                    ) {
                        if visible.is_empty() {
                            View(height: Constraint::Length(1)) {
                                Text(text: Line::from("no match, press Esc to reset").dark_gray().centered())
                            }
                        } else {
                            for (index, line) in list_lines.into_iter().enumerate() {
                                View(height: Constraint::Length(1), key: index) {
                                    Text(text: line)
                                }
                            }
                        }
                    }
                    Border(
                        width: Constraint::Fill(1),
                        flex_direction: Direction::Vertical,
                        border_style: Style::new().fg(Color::Cyan),
                        top_title: Line::from(" state ").fg(Color::Cyan).centered(),
                    ) {
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from(format!("selected: {selected_name}")).centered())
                        }
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from(format!(
                                "query: {}",
                                if current_query.is_empty() {
                                    "<empty>".to_string()
                                } else {
                                    current_query.clone()
                                }
                            )).centered())
                        }
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from(format!("matches: {visible_len}")).centered())
                        }
                        View(height: Constraint::Length(1)) {
                            Text(text: Line::from(format!("chosen: {chosen_view}")).centered())
                        }
                    }
                }
            }
        }
    )
}
