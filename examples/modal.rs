use ratatui::{layout::Constraint, text::Line, widgets::Padding};
use ratatui_kit::{
    AnyElement, ElementExt, Hooks, component, element,
    prelude::{border::Border, modal::Modal, view::View},
    ratatui::style::{Style, Stylize},
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
    hooks.use_future(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            state += 1;
        }
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
                open:true,
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
