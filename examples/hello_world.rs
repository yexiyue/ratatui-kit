use ratatui::layout::Alignment;
use ratatui_kit::prelude::*;
use ratatui_kit::ratatui;

#[tokio::main]
async fn main() {
    element!(Border{
        Text(text: "Hello, World!", alignment: Alignment::Center)
    })
    .fullscreen()
    .await
    .expect("Failed to run the application");
}
