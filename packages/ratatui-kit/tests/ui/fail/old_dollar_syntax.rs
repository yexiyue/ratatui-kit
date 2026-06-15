//! 旧 `$` 适配器语法已移除：应给出迁移提示。
#![allow(unused)]
use ratatui_kit::prelude::*;
use ratatui_kit::ratatui::text::Line;

fn main() {
    let _ = element!(View {
        $Line::from("x")
    });
}
