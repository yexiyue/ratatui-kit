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
    element!(SelectText)
        .fullscreen()
        .await
        .expect("Failed to run the application");
}

#[component]
fn SelectText(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let state = hooks.use_state(ListState::default);

    hooks.use_events(move |event| {
        if let Event::Key(key) = event
            && key.kind == KeyEventKind::Press
        {
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
    });

    let list_props = MyListProps { state };

    element!(
        Border(
            flex_direction: Direction::Vertical,
            justify_content: Flex::Center,
        ){
            MyList(..list_props){}

        }
    )
}

pub struct MyList {
    state: State<ListState>,
}

#[derive(Debug, Props)]
pub struct MyListProps {
    pub state: State<ListState>,
}

impl Component for MyList {
    type Props<'a> = MyListProps;

    fn new(props: &Self::Props<'_>) -> Self {
        Self { state: props.state }
    }

    fn update(
        &mut self,
        props: &mut Self::Props<'_>,
        _hooks: Hooks,
        _updater: &mut ComponentUpdater,
    ) {
        self.state = props.state;
    }
    fn draw(&mut self, drawer: &mut ComponentDrawer<'_, '_>) {
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

        drawer.render_stateful_widget(list, drawer.area, &mut self.state.write_no_update());
    }
}
