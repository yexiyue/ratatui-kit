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

type FnBox = Box<dyn FnOnce(&mut Buffer) + Send>;

#[derive(Clone, Default)]
pub struct InsertBeforeHandler {
    queue: Arc<Mutex<VecDeque<(u16, FnBox)>>>,
    waker: Arc<Mutex<Option<Waker>>>,
}

impl Hook for InsertBeforeHandler {
    fn poll_change(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context,
    ) -> std::task::Poll<()> {
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
        let mut queue = self.queue.lock().unwrap();
        for (height, callback) in queue.drain(..) {
            updater.terminal().insert_before(height, callback);
        }
    }
}

impl InsertBeforeHandler {
    pub fn insert_before<F>(&self, height: u16, callback: F) -> &Self
    where
        F: FnOnce(&mut Buffer) + Send + 'static,
    {
        let mut queue = self.queue.lock().unwrap();
        queue.push_back((height, Box::new(callback)));
        self
    }

    pub fn render_before<T: Widget + Send + 'static>(&self, widget: T, height: u16) -> &Self {
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
    fn use_insert_before(&mut self) -> InsertBeforeHandler;
}

impl UseInsertBefore for Hooks<'_, '_> {
    fn use_insert_before(&mut self) -> InsertBeforeHandler {
        self.use_hook(InsertBeforeHandler::default).clone()
    }
}
