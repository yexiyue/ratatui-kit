//! Blockquote 内置组件示例。

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
    let mut depth = hooks.use_state(|| 1u32);
    let mut exit = hooks.use_exit();

    hooks.use_event_handler(EventScope::Current, EventPriority::Normal, move |event| {
        if let Event::Key(key) = event
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Char('d') | KeyCode::Right => {
                    depth.set((depth.get() % 3) + 1);
                    return EventResult::Consumed;
                }
                KeyCode::Left => {
                    depth.set(if depth.get() == 1 { 3 } else { depth.get() - 1 });
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

    element!(
        Center(width: Constraint::Length(60), height: Constraint::Length(22)) {
            Border(
                flex_direction: Direction::Vertical,
                gap: 1,
                border_style: Style::new().blue(),
                top_title: Line::from(" blockquote component ").blue().bold().centered(),
                bottom_title: Line::from(" <- -> depth | q quit ").dark_gray().centered(),
            ) {
                View(height: Constraint::Length(1)) {
                    Text(text: Line::from(format!("depth: {}", depth.get())))
                }

                Divider(char: '─', style_cfg: Style::new().fg(Color::DarkGray))

                Text(text: Line::from("depth = 1:"))
                Blockquote(depth: 1, prefix_color: Color::DarkGray) {
                    Text(text: Line::from("first line of quoted text"))
                    Text(text: Line::from("second line of quoted text"))
                }

                Text(text: Line::from("depth = 2:"))
                Blockquote(depth: 2, prefix_color: Color::DarkGray) {
                    Text(text: Line::from("nested level-2 quote"))
                    Text(text: Line::from("content flows here"))
                }

                Text(text: Line::from("depth = 3:"))
                Blockquote(depth: 3, prefix_color: Color::DarkGray) {
                    Text(text: Line::from("deeply nested quote"))
                }
            }
        }
    )
}
