use ratatui::{layout::Constraint, text::Line};
use ratatui_kit::{
    AnyElement, ElementExt, Hooks, component, element,
    prelude::{border::Border, view::View},
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
            View(

                height:Constraint::Length(10),
                justify_content:ratatui::layout::Flex::End,
                flex_direction:ratatui::layout::Direction::Vertical,
            ){
                View(height:Constraint::Length(1)){
                    $line.clone()
                }
            }
            View(){
                Border(
                    border_style:Style::default().blue(),
                    style:Style::default().on_white(),
                    width:Constraint::Length(20)
                ){
                    $line
                }
            }
        }
    )
}
