use ratatui::{
    style::{Style, Stylize},
    text::Line,
};
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
    hooks.use_future(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            state += 1;
        }
    });

    element!(
        $Line::styled(
            format!("Counter: {}", state),
            Style::default().fg(ratatui::style::Color::Green).bold(),
        )
        .centered()
        .bold()
        .underlined()
    )
}
