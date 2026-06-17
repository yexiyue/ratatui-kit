// Bug 修复:`widget(expr)` 应接纳**仅实现按值 `Widget`**(不实现 `&T: Widget`)的部件。
// 旧 `WidgetAdapter` 约束 `for<'a> &'a T: Widget` 把这类部件(如 `tui-big-text` 的 `BigText`)
// 挡在外面;修复后 adapter 改 `T: Widget + Clone` 按值渲染,应可用。
//
// 边界覆盖:按值-only 自定义部件、同时实现按值+按引用的内置部件(Line/Paragraph)、
// 在一等控制流分支内 `widget(...)`。
#![allow(dead_code)]

use ratatui_kit::prelude::*;
use ratatui_kit::ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::Line,
    widgets::{Paragraph, Widget},
};

// 仅实现按值 `Widget`、**不**实现 `&ByValueOnly: Widget` —— 模拟 BigText 这类部件。
#[derive(Clone)]
struct ByValueOnly(&'static str);

impl Widget for ByValueOnly {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Paragraph::new(self.0).render(area, buf);
    }
}

#[component]
fn App(mut _hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let cond = true;

    element!(View {
        // 按值-only 部件(本修复的核心):旧约束下不可用
        widget(ByValueOnly("by-value-only"))

        // 同时按值+按引用的内置部件仍可用(回归保护)
        widget(Line::from("line"))
        widget(Paragraph::new("paragraph"))

        // 控制流分支内 widget(...) 亦可
        if cond {
            widget(ByValueOnly("in-if"))
        } else {
            widget(Line::from("in-else"))
        }
    })
}

fn main() {
    let _ = element!(App);
}
