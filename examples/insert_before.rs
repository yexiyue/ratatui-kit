use ratatui::{
    style::{Style, Stylize},
    text::Line,
};
use ratatui_kit::ratatui::{
    self,
    widgets::{Paragraph, Widget},
};
use ratatui_kit::{prelude::*, ratatui::TerminalOptions};

#[tokio::main]
async fn main() {
    element!(Counter)
        .into_any()
        .render_loop(TerminalOptions {
            viewport: ratatui::Viewport::Inline(10),
        })
        .await
        .expect("Failed to run the application");
}

#[component]
fn Counter(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut state = hooks.use_state(|| 0);
    let handle = hooks.use_insert_before();
    hooks.use_future(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            state += 1;
            handle.insert_before(1, move |buf| {
                Paragraph::new(format!("Counter: {}", state)).render(buf.area, buf);
            });
        }
    });

    element!(
        $Line::styled(
            format!("Counter: {state}"),
            Style::default().fg(ratatui::style::Color::Green).bold(),
        )
        .centered()
        .bold()
        .underlined()
    )
}
