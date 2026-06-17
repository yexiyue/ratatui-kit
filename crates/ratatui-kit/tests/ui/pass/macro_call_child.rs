// Bug 修复:宏调用 `path!(...)`(典型是嵌套的 `element!(...)`,也含 `vec![...]`)在子节点位
// 应等同 `{ expr }` embed 解析。修复前,容器 children 块 / 一等控制流分支体内的 `element!(...)`
// 会被当成「组件头 `element` + 剩余 `!(...)`」→ 误报 `expected identifier`。
//
// 边界覆盖:裸宏子节点、链式 `.into_any()`、if/else-if/else、match(含 `|` 与 guard)、for、
// vec![]、与原生组件/`widget()` 适配器混排(确保组件头与适配器识别不被新分支影响)。
#![allow(dead_code)]

use ratatui_kit::prelude::*;
use ratatui_kit::ratatui::text::Line;

#[component]
fn App(mut _hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let cond = true;
    let maybe: Option<u8> = Some(1);
    let n = 2u8;

    element!(View {
        // 裸 element!(...) 作子节点
        element!(Text(text: "bare-macro-child"))

        // element!(...) 链式 `.into_any()` 作子节点
        element!(Text(text: "chained")).into_any()

        // 一等控制流分支体内放 element!(...)(分支体即 children 块)
        if cond {
            element!(Border { Text(text: "in-if") })
        } else {
            element!(Text(text: "in-else")).into_any()
        }

        // if let + else if 链,分支用 element!(...)
        if let Some(v) = maybe {
            element!(Text(text: format!("some {v}")))
        } else if cond {
            element!(Text(text: "elif"))
        } else {
            element!(Text(text: "none"))
        }

        // match:`|` 模式 + guard,分支体内 element!(...)
        match n {
            0 | 1 => { element!(Text(text: "low")) }
            x if x < 5 => { element!(Text(text: "mid")).into_any() }
            _ => { element!(Border { Text(text: "high") }) }
        }

        // for 体内 element!(...)(每项给 key)
        for i in 0..2u8 {
            element!(Text(text: format!("row {i}"), key: i))
        }

        // vec![] 宏作子节点(返回 Vec<AnyElement>)
        vec![
            element!(Text(text: "a")).into_any(),
            element!(Text(text: "b")).into_any(),
        ]

        // 路径限定宏 `ratatui_kit::element!(...)` 作子节点:锁定 is_macro_call 用 `fork.parse::<syn::Path>()`
        // 解析**多段路径**的 load-bearing 行为。若有人把它退化成单段 `peek(Ident) && peek2(Token![!])`,
        // 此处第二个 token 是 `::` 而非 `!` → 误判为组件头 → 编译失败,从而捕获回归。
        ratatui_kit::element!(Text(text: "path-qualified-macro"))

        // 与原生组件子节点、`widget()` 适配器、`{ expr }` embed 混排——均不受新分支影响
        Text(text: "native-component")
        widget(Line::from("native-widget"))
        { Some(element!(Text(text: "brace-embed"))) }
    })
}

fn main() {
    let _ = element!(App);
}
