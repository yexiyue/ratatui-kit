//! `stateful(...)` 只接受 `widget, state`：多参应报错。
#![allow(unused)]
use ratatui_kit::prelude::*;
use ratatui_kit::ratatui::text::Line;

fn main() {
    let _ = element!(View {
        stateful(Line::from("a"), b, c)
    });
}
