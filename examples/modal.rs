use ratatui::{
    layout::Constraint,
    style::{Style, Stylize},
    text::Line,
    widgets::Padding,
};
use ratatui_kit::crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui_kit::prelude::*;

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
        View{
            View{
                Border(
                    border_style:Style::default().blue(),
                    top_title:Some(Line::from("Counter")),
                    bottom_title:Some(Line::from("Press Ctrl+C to exit").centered()),
                    ..Default::default()
                ){
                    $line.clone()
                }
            }
            Modal(
                open:open.get(),
                width:Constraint::Percentage(50),
                height:Constraint::Percentage(50),
                style:Style::default().dim(),
                ..Default::default()
            ){
                Border(
                    top_title:Some(Line::from("Modal Title").centered().yellow()),
                    padding:Padding::new(4,4,1,1),
                    ..Default::default()
                ) {
                    View(height:Constraint::Length(1),..Default::default()){
                        $line
                    }
                }
            }
        }
    )
}
