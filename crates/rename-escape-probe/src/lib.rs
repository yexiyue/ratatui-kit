//! 1.6 rename 逃生舱护栏。
//!
//! 第三方作者若用 `cargo` 把 `ratatui-kit` 依赖 rename(这里 key = `rk`,
//! `package = "ratatui-kit"`),宏展开的 `::ratatui_kit::...` 绝对路径就找不到 crate。
//! 逃生办法是 Rust 惯例的 `extern crate <renamed> as ratatui_kit;`,把它重命名回来。
//!
//! 本 crate 刻意 rename 依赖并使用该逃生舱 + 全套宏(`#[component]` / `element!` /
//! `#[derive(Props)]` / `#[with_layout_style]`);随 `--workspace` 编译,逃生舱一旦失效即红。

extern crate rk as ratatui_kit;

use ratatui_kit::prelude::*;

#[with_layout_style]
#[derive(Default, Props)]
pub struct PanelProps<'a> {
    pub children: Vec<AnyElement<'a>>,
}

pub struct Panel;

impl Component for Panel {
    type Props<'a> = PanelProps<'a>;

    fn new(_props: &Self::Props<'_>) -> Self {
        Self
    }

    fn update(
        &mut self,
        props: &mut Self::Props<'_>,
        _hooks: Hooks,
        updater: &mut ComponentUpdater,
    ) {
        updater.set_layout_style(props.layout_style());
        updater.update_children(&mut props.children, None);
    }
}

#[component]
pub fn App(mut _hooks: Hooks) -> impl Into<AnyElement<'static>> {
    element!(Panel(gap: 1) {
        Panel()
    })
}
