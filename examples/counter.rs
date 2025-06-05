use ratatui::text::Line;
use ratatui_kit::{
    AnyElement, ElementExt, Hooks, component, element,
    prelude::fragment::Fragment,
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
        Fragment{
            $line
        }
    )
}
