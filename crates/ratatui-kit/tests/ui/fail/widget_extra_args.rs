//! `widget(...)` 只接受一个表达式：多参应报错。
#![allow(unused)]
use ratatui_kit::prelude::*;
use ratatui_kit::ratatui::text::Line;

fn main() {
    let _ = element!(View {
        widget(Line::from("a"), Line::from("b"))
    });
}
