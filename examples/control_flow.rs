//! 验证 element! 一等控制流（if / if let / else if / for / match）。
//!
//! 关键点：各分支可返回**不同元素类型**，无需 `.into_any()` 统一类型。
//! 运行：`cargo run --example control_flow`

use ratatui_kit::{
    prelude::*,
    ratatui::{
        layout::{Constraint, Direction},
        style::Style,
        widgets::Borders,
    },
};

#[tokio::main]
async fn main() {
    element!(ControlFlowDemo)
        .fullscreen()
        .await
        .expect("Failed to run the application");
}

#[component]
fn ControlFlowDemo(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let count = hooks.use_state(|| 3u8);
    let maybe_name: Option<&str> = Some("ratatui-kit");
    let items = ["alpha", "beta", "gamma"];

    element!(
        Border(
            flex_direction: Direction::Vertical,
            gap: 1,
        ) {
            // if / else：两个分支是不同元素类型（一个 Text，一个 Border 包 Text）
            if count.get() % 2 == 0 {
                Text(text: "count 是偶数", style: Style::new().green())
            } else {
                Border(borders: Borders::ALL) {
                    Text(text: "count 是奇数（包了个边框，证明分支类型可不同）")
                }
            }

            // if let + else if 链
            if let Some(name) = maybe_name {
                Text(text: format!("名字: {name}"))
            } else if count.get() > 0 {
                Text(text: "有 count 无名字")
            } else {
                Text(text: "都没有")
            }

            // for：列表内联，每项给稳定 key
            View(height: Constraint::Length(items.len() as u16)) {
                for (i, item) in items.iter().enumerate() {
                    Text(text: format!("  {i}. {item}"), key: i)
                }
            }

            // match：每个分支用 {} 包裹，可带不同元素
            match count.get() {
                0 => { Text(text: "zero") }
                n if n < 5 => { Text(text: format!("small: {n}"), style: Style::new().yellow()) }
                _ => { Text(text: "big") }
            }
        }
    )
}
