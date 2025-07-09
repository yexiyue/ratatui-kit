use ratatui_kit::{
    prelude::*,
    ratatui::layout::Direction,
    ratatui::{
        layout::{Constraint, Flex},
        style::{Style, Stylize},
        text::Line,
    },
};

#[tokio::main]
async fn main() {
    element!(Counter)
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

    element!(
        Border(
            flex_direction: Direction::Vertical,
            justify_content: Flex::Center,
        ){
            View(height:Constraint::Length(1)){
                $Line::styled(
                    format!("Counter: {state}"),
                    Style::default().green().bold(),
                )
                .centered()
                .bold()
            }
        }
    )
}
