use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    task::{Poll, Waker},
};

use ratatui::{buffer::Buffer, widgets::Widget};

use crate::{Hook, Hooks, Terminal};

mod private {
    pub trait Sealed {}
    impl Sealed for crate::hooks::Hooks<'_, '_> {}
}

type FnBox = Box<dyn FnOnce(&mut Buffer)>;

#[derive(Clone, Default)]
pub struct InsertBeforeHandler {
    queue: Arc<Mutex<VecDeque<(u16, FnBox)>>>,
    waker: Arc<Mutex<Option<Waker>>>,
}

impl Hook for InsertBeforeHandler {
    fn poll_change(&mut self, cx: &mut std::task::Context) -> std::task::Poll<()> {
        let mut waker = self.waker.lock().unwrap();
        let mut queue = self.queue.lock().unwrap();
        if queue.is_empty() {
            waker.replace(cx.waker().clone());
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }

    fn post_component_update(&mut self, updater: &mut crate::ComponentUpdater) {
        // 同步绘制回调可以再次排队;执行前释放锁,新增任务留到下一次更新。
        let pending = std::mem::take(&mut *self.queue.lock().unwrap());
        for (height, callback) in pending {
            updater.terminal().insert_before(height, callback);
        }
    }
}

impl InsertBeforeHandler {
    pub fn insert_before<F>(&self, height: u16, callback: F) -> &Self
    where
        F: FnOnce(&mut Buffer) + 'static,
    {
        let mut queue = self.queue.lock().unwrap();
        queue.push_back((height, Box::new(callback)));
        self
    }

    pub fn render_before<T: Widget + 'static>(&self, widget: T, height: u16) -> &Self {
        self.insert_before(height, move |buf| {
            widget.render(buf.area, buf);
        });
        self
    }

    pub fn finish(&self) {
        if let Some(waker) = self.waker.lock().unwrap().take() {
            waker.wake();
        }
    }
}

pub trait UseInsertBefore: private::Sealed {
    // 在终端渲染区域前插入内容。
    fn use_insert_before(&mut self) -> InsertBeforeHandler;
}

impl UseInsertBefore for Hooks<'_, '_> {
    fn use_insert_before(&mut self) -> InsertBeforeHandler {
        self.use_hook(InsertBeforeHandler::default).clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ComponentUpdater, ElementKey, component::Components, context::ContextStack,
        layout_style::LayoutStyle, terminal::UpdaterTerminal,
    };
    use ratatui::layout::Rect;
    use std::io;

    #[derive(Default)]
    struct SynchronousTerminal {
        calls: usize,
    }

    impl UpdaterTerminal for SynchronousTerminal {
        fn insert_before(
            &mut self,
            height: u16,
            draw: Box<dyn FnOnce(&mut Buffer)>,
        ) -> io::Result<()> {
            self.calls += 1;
            draw(&mut Buffer::empty(Rect::new(0, 0, 10, height)));
            Ok(())
        }
    }

    #[test]
    fn insert_before_callback_can_enqueue_for_the_next_update() {
        let mut handler = InsertBeforeHandler::default();
        let reentrant = handler.clone();
        handler.insert_before(1, move |_| {
            // 旧实现直接失败,不让回归测试永久卡在重入 lock。
            assert!(reentrant.queue.try_lock().is_ok());
            reentrant.insert_before(2, |_| {});
        });
        let mut root = ();
        let mut stack = ContextStack::root(&mut root);
        let mut terminal = SynchronousTerminal::default();
        let mut components = Components::default();
        let mut layout = LayoutStyle::default();
        {
            let mut updater = ComponentUpdater::new(
                ElementKey::decl(0),
                &mut stack,
                &mut terminal,
                &mut components,
                &mut layout,
            );
            handler.post_component_update(&mut updater);
            assert_eq!(handler.queue.lock().unwrap().len(), 1);
        }
        assert_eq!(terminal.calls, 1);
        {
            let mut updater = ComponentUpdater::new(
                ElementKey::decl(0),
                &mut stack,
                &mut terminal,
                &mut components,
                &mut layout,
            );
            handler.post_component_update(&mut updater);
        }
        assert_eq!(terminal.calls, 2);
        assert!(handler.queue.lock().unwrap().is_empty());
    }
}
