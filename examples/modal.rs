use ratatui::{layout::Constraint, text::Line, widgets::Padding};
use ratatui_kit::{
    AnyElement, ElementExt, Hooks, component,
    crossterm::event::{Event, KeyCode, KeyEventKind},
    element,
    prelude::{border::Border, modal::Modal, view::View},
    ratatui::style::{Style, Stylize},
    use_events::UseEvents,
    use_future::UseFuture,
    use_state::UseState,
};

#[tokio::main]
async fn main() {
    element!(Counter)
        .into_any()
        .fullscreen()
        .await
        .expect("Failed to run the application");
}

#[component]
fn Counter(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut state = hooks.use_state(|| 0);
    let mut open = hooks.use_state(|| false);
    hooks.use_future(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            state += 1;
        }
    });

    hooks.use_events(move |event| match event {
        Event::Key(key_event) => {
            if key_event.kind == KeyEventKind::Press && key_event.code == KeyCode::Tab {
                open.set(!open.get());
            }
        }
        _ => {}
    });

    let line = Line::styled(
        format!("Counter: {}", state),
        Style::default().fg(ratatui::style::Color::Green).bold(),
    )
    .centered()
    .bold()
    .underlined();

    element!(
        View(
            justify_content:ratatui::layout::Flex::SpaceAround,
            flex_direction:ratatui::layout::Direction::Horizontal,
        ){
            View{
                Border(
                    border_style:Style::default().blue(),
                    top_title:Some(Line::from("Counter")),
                    bottom_title:Some(Line::from("Press Ctrl+C to exit").centered()),
                ){
                    $line.clone()
                }
            }
            Modal(
                open:open.get(),
                width:Constraint::Percentage(50),
                height:Constraint::Percentage(50),
                style:Style::default().dim(),
            ){
                Border(
                    top_title:Some(Line::from("Modal Title").centered().yellow()),
                    padding:Padding::new(4,4,1,1),
                ) {
                    View(height:Constraint::Length(1)){
                        $line
                    }
                }
            }
        }
    )
}
