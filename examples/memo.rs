use ratatui::{
    style::{Style, Stylize},
    text::Line,
};
use ratatui_kit::prelude::*;
use ratatui_kit::ratatui;

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
    let double_counter = hooks.use_memo(|| state.get() * 2, state.get());

    hooks.use_future(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            state += 1;
        }
    });

    element!(
        $Line::styled(
            format!("Counter: {}", double_counter),
            Style::default().fg(ratatui::style::Color::Green).bold(),
        )
        .centered()
        .bold()
        .underlined()
    )
}
