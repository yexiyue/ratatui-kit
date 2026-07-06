use std::{cell::Cell, rc::Rc};

use crossterm::{event::Event, terminal};
use ratatui::layout::Rect;

use crate::{
    Hook, State, SystemContext, UseState,
    input::{EventOptions, EventPriority, EventResult},
};

mod private {
    pub trait Sealed {}
    impl Sealed for crate::Hooks<'_, '_> {}
}

// 获取终端当前尺寸，并在终端尺寸变化时更新，适合响应式布局。
pub trait UseTerminalSize: private::Sealed {
    fn use_terminal_size(&mut self) -> (u16, u16);
}

impl UseTerminalSize for crate::Hooks<'_, '_> {
    fn use_terminal_size(&mut self) -> (u16, u16) {
        let mut size = self.use_state(|| terminal::size().unwrap_or((0, 0)));

        let hook = self.use_hook(UseTerminalSizeImpl::new);
        hook.size = Some(size);

        size.get()
    }
}

// 终端尺寸监听保留为专用 hook，而不是复用 `use_event_handler(Global, ..)`。
//
// 这样手写 Component 直接调用 `use_terminal_size` 时仍不需要先 `with_context_stack`;
// 注册 Resize handler 时由 `post_component_update` 通过 updater 拿根 `SystemContext`。
struct UseTerminalSizeImpl {
    size: Option<State<(u16, u16)>>,
}

impl UseTerminalSizeImpl {
    fn new() -> Self {
        Self { size: None }
    }
}

impl Hook for UseTerminalSizeImpl {
    fn post_component_update(&mut self, updater: &mut crate::ComponentUpdater) {
        let Some(mut size) = self.size else {
            return;
        };
        let mut system = updater
            .get_context_mut::<SystemContext>()
            .expect("`SystemContext` missing (the root context always provides it)");

        // Resize 是真全局事件：`layer=None` 不被任何 blocks_lower 截断,返回 Ignored
        // 让多个 use_terminal_size 订阅者都能收到。
        system.input.register_handler(
            None,
            EventPriority::Normal,
            EventOptions::default(),
            Rc::new(Cell::new(Rect::default())),
            Box::new(move |event| {
                if let Event::Resize(width, height) = event {
                    size.set((width, height));
                }
                EventResult::Ignored
            }),
        );
    }
}

// 获取组件当前尺寸，但是是上一帧的尺寸
pub trait UsePreviousSize: private::Sealed {
    fn use_previous_size(&mut self) -> Rect;
}

#[doc(hidden)]
pub struct UsePreviousSizeImpl {
    size: Rect,
}

impl Default for UsePreviousSizeImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl UsePreviousSizeImpl {
    pub fn new() -> Self {
        UsePreviousSizeImpl {
            size: Rect::default(),
        }
    }
}

impl Hook for UsePreviousSizeImpl {
    fn pre_component_draw(&mut self, drawer: &mut crate::ComponentDrawer) {
        self.size = drawer.area;
    }
}

impl UsePreviousSize for crate::Hooks<'_, '_> {
    fn use_previous_size(&mut self) -> Rect {
        let hook = self.use_hook(UsePreviousSizeImpl::new);
        hook.size
    }
}
