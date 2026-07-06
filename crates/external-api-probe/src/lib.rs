//! 宏 hygiene 回归护栏 / 最小「外部组件 crate」示例。
//!
//! 这个 crate **只**依赖 `ratatui-kit`(刻意不依赖 `ratatui`/`crossterm`),模拟第三方
//! 组件作者的视角:只通过公共扩展 API 定义组件与自定义 hook。它随 `--workspace` 自动
//! 编译,因此一旦过程宏展开回退成裸 `ratatui::`/`crossterm::` 路径(外部作用域解析不到),
//! 这里就会编译失败、CI 变红。
//!
//! 为什么不用 trybuild:`tests/ui/pass` 的临时 crate 会 mirror 被测 crate 的
//! `ratatui`/`crossterm` 依赖(作用域里有裸 `ratatui`),故抓不到 hygiene 回退。
//!
//! 覆盖:`#[with_layout_style]` + `#[derive(Props)]` + 手动 `impl Component` +
//! `#[component]` + `element!` + `use_state` + 自定义 `Hook` / `use_hook`。

use ratatui_kit::prelude::*;
use ratatui_kit::ratatui::layout::Direction;

// ── 1) #[derive(Props)] + #[with_layout_style] + 带 children 的 Props ──────────
#[with_layout_style]
#[derive(Default, Props)]
pub struct PanelProps<'a> {
    pub title: String,
    pub children: Vec<AnyElement<'a>>,
}

// ── 2) 手动 impl Component:验证 Component / ComponentUpdater / AnyElement /
//        update_children / set_layout_style / layout_style() 都对外可达 ──────────
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

// ── 3) #[component] 函数组件 + element! + use_state + 自定义 hook ───────────────
#[derive(Default, Props)]
pub struct BadgeProps {
    pub label: String,
}

#[component]
pub fn Badge(props: &BadgeProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut n = hooks.use_state(|| 0u32);
    n += 1;
    let ticks = hooks.use_tick();
    let title = format!("{} #{} ({}t)", props.label, n.get(), ticks);
    element! {
        Panel(title: title, flex_direction: Direction::Vertical, gap: 1)
    }
}

// ── 4) 自定义 hook:实现 Hook trait + 定义(未 seal 的)extension trait ───────────
//        验证「外部无法用 private::Sealed,但仍能通过 use_hook 注册自定义 hook」。
pub struct TickHook {
    n: u32,
}

impl Hook for TickHook {
    fn poll_change(&mut self, _cx: &mut std::task::Context) -> std::task::Poll<()> {
        std::task::Poll::Pending
    }
}

pub trait UseTick {
    fn use_tick(&mut self) -> u32;
}

impl UseTick for Hooks<'_, '_> {
    fn use_tick(&mut self) -> u32 {
        let h = self.use_hook(|| TickHook { n: 0 });
        h.n += 1;
        h.n
    }
}
