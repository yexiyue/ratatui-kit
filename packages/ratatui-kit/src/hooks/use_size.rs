use crossterm::{event::Event, terminal};
use ratatui::layout::Rect;

use crate::{Hook, UseEvents, UseState};

mod private {
    pub trait Sealed {}
    impl Sealed for crate::Hooks<'_, '_> {}
}

/// 获取终端当前尺寸，并在终端尺寸变化时更新，适合响应式布局。
pub trait UseTerminalSize: private::Sealed {
    fn use_terminal_size(&mut self) -> (u16, u16);
}

impl UseTerminalSize for crate::Hooks<'_, '_> {
    fn use_terminal_size(&mut self) -> (u16, u16) {
        let mut size = self.use_state(|| terminal::size().unwrap_or((0, 0)));

        self.use_events(move |event| {
            if let Event::Resize(width, height) = event {
                size.set((width, height));
            }
        });

        size.get()
    }
}

/// 获取组件当前尺寸，但是是上一帧的尺寸
pub trait UsePreviousSize: private::Sealed {
    fn use_previous_size(&mut self) -> Rect;
}

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
