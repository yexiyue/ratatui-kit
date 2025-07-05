use ratatui::text::Line;
use ratatui_kit::prelude::*;
use ratatui_kit::ratatui;

#[tokio::main]
async fn main() {
    element!(Border{
        $Line::from("Hello, World!").centered()
    })
    .fullscreen()
    .await
    .expect("Failed to run the application");
}
