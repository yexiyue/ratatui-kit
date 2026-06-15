//! 旧 `#(expr)` 子节点语法已移除：应给出迁移提示。
use ratatui_kit::prelude::*;

fn main() {
    let value = Some(element!(Text(text: "x")));
    let _ = element!(View {
        #(value)
    });
}
