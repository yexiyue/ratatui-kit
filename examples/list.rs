use ratatui_kit::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    ratatui::{
        layout::{Direction, Flex},
        style::{Color, Style},
        widgets::{Block, Borders, List, ListItem, ListState},
    },
};

#[tokio::main]
async fn main() {
    element!(StatefulList)
        .fullscreen()
        .await
        .expect("Failed to run the application");
}

#[component]
fn StatefulList(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let items = vec![
        ListItem::new("Item 1"),
        ListItem::new("Item 2"),
        ListItem::new("Item 3"),
        ListItem::new("Item 4"),
        ListItem::new("Item 5"),
    ];

    let list = List::new(items)
        .block(Block::default().title("Select Text").borders(Borders::ALL))
        .highlight_style(Style::default().bg(Color::Blue).fg(Color::White))
        .highlight_symbol(">> ");

    let state = hooks.use_state(ListState::default);

    hooks.use_events(move |event| match event {
        Event::Key(key) => {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Up => {
                        state.write().select_previous();
                    }
                    KeyCode::Down => {
                        state.write().select_next();
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    });

    element!(
        Border(
            flex_direction: Direction::Vertical,
            justify_content: Flex::Center,
        ){
            $(list.clone(),state)
        }
    )
}
