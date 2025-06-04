use ratatui::{
    layout::Alignment,
    style::{Style, Stylize},
};
use ratatui_kit::{
    AnyElement, ElementExt, Hooks, NoProps, Props, component, element,
    text::{Text, TextProps},
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

    element!(Text(
        text: format!("Hello World {}", state),
        alignment: Some(Alignment::Center),
        style:Style::default().yellow(),
    ))
}
