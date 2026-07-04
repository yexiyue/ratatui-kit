//! Divider 内置组件示例。

use ratatui_kit::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    ratatui::{
        layout::{Constraint, Direction},
        style::{Color, Style, Stylize},
        text::Line,
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
    let mut idx = hooks.use_state(|| 0usize);
    let mut exit = hooks.use_exit();

    hooks.use_event_handler(EventScope::Current, EventPriority::Normal, move |event| {
        if let Event::Key(key) = event
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Right => {
                    idx.set((idx.get() + 1) % 4);
                    return EventResult::Consumed;
                }
                KeyCode::Left => {
                    idx.set(if idx.get() == 0 { 3 } else { idx.get() - 1 });
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

    let chars = ['─', '━', '═', '·'];
    let labels = ["default", "thick", "double", "dot"];
    let colors = [Color::DarkGray, Color::Cyan, Color::Yellow, Color::Red];
    let i = idx.get();

    element!(
        Center(width: Constraint::Length(60), height: Constraint::Length(10)) {
            Border(
                flex_direction: Direction::Vertical,
                gap: 1,
                border_style: Style::new().blue(),
                top_title: Line::from(" divider component ").blue().bold().centered(),
                bottom_title: Line::from(" <- -> switch | q quit ").dark_gray().centered(),
            ) {
                View(height: Constraint::Length(1)) {
                    Text(text: Line::from(format!("style: {} '{}'", labels[i], chars[i])))
                }
                Divider(char: chars[i], style_cfg: Style::new().fg(colors[i]))
            }
        }
    )
}
