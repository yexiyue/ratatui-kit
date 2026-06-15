//! 新 DSL 应编译通过：一等控制流、widget()/stateful 之 widget()、{ expr }、Text。
#![allow(dead_code)]

use ratatui_kit::prelude::*;
use ratatui_kit::ratatui::text::Line;

#[component]
fn App(mut _hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let cond = true;
    let items = [1u8, 2, 3];

    element!(View {
        // if / else：分支返回不同元素类型，无需 into_any
        if cond {
            Text(text: "yes")
        } else {
            Border { Text(text: "no") }
        }

        // if let + else if 链
        if let Some(first) = items.first() {
            Text(text: format!("first = {first}"))
        } else if cond {
            Text(text: "fallback")
        }

        // for：列表内联，每项给 key
        for (i, x) in items.iter().enumerate() {
            Text(text: format!("{x}"), key: i)
        }

        // match：分支体用 {} 包裹
        match items.len() {
            0 => { Text(text: "empty") }
            n if n < 5 => { Text(text: "few") }
            _ => { Text(text: "many") }
        }

        // 原生 widget 适配器
        widget(Line::from("native"))

        // { expr }：内嵌任意返回 Element/Option/Vec/Iterator 的表达式
        { Some(element!(Text(text: "embedded"))) }
    })
}

fn main() {
    let _ = element!(App);
}
