//! 宏 hygiene 护栏(以 example 形式)。
//!
//! `ratatui-kit-examples` 这个根 crate **不直接依赖** `ratatui` / `crossterm`,所以站在
//! 「只有 ratatui-kit 在场」的外部作者视角:一旦过程宏展开回退成裸 `ratatui::` /
//! `crossterm::` 路径(或泄漏未导出的项),本 example 就编译失败、CI 变红。
//! `cargo test --examples` 会编译它,故无需独立 crate。
//!
//! 覆盖:`#[with_layout_style]` + `#[derive(Props)]` + 手动 `impl Component` +
//! `#[component]` + `element!` + `use_state` + 自定义 `Hook` / `use_hook`。
//!
//! 注:依赖被 rename 时的逃生舱(`extern crate <renamed> as ratatui_kit;`)是 Rust 语言
//! 机制,用法见 `EXTENSION_API.md` / `COMPONENT_GUIDE.md`。

#![allow(dead_code)]

use ratatui_kit::prelude::*;
use ratatui_kit::ratatui::layout::Direction;

#[with_layout_style]
#[derive(Default, Props)]
struct PanelProps<'a> {
    title: String,
    children: Vec<AnyElement<'a>>,
}

struct Panel;

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

#[derive(Default, Props)]
struct BadgeProps {
    label: String,
}

#[component]
fn Badge(props: &BadgeProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut n = hooks.use_state(|| 0u32);
    n += 1;
    let ticks = hooks.use_tick();
    let title = format!("{} #{} ({}t)", props.label, n.get(), ticks);
    element!(Panel(title: title, flex_direction: Direction::Vertical, gap: 1))
}

struct TickHook {
    n: u32,
}

impl Hook for TickHook {
    fn poll_change(&mut self, _cx: &mut std::task::Context) -> std::task::Poll<()> {
        std::task::Poll::Pending
    }
}

trait UseTick {
    fn use_tick(&mut self) -> u32;
}

impl UseTick for Hooks<'_, '_> {
    fn use_tick(&mut self) -> u32 {
        let h = self.use_hook(|| TickHook { n: 0 });
        h.n += 1;
        h.n
    }
}

fn main() {
    // 仅构造以驱动宏展开的编译验证;不进入渲染循环。
    let _tree = element!(Panel(flex_direction: Direction::Vertical, gap: 1) {
        Badge(label: "probe".to_string())
    });
}
