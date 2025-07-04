use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use ratatui::buffer::Buffer;

use crate::{Hook, Hooks, Terminal};

mod private {
    pub trait Sealed {}
    impl Sealed for crate::hooks::Hooks<'_, '_> {}
}

type FnBox = Box<dyn FnOnce(&mut Buffer) + Send>;

#[derive(Clone, Default)]
pub struct InsertBeforeHandler {
    queue: Arc<Mutex<VecDeque<(u16, FnBox)>>>,
}

impl Hook for InsertBeforeHandler {
    fn post_component_update(&mut self, updater: &mut crate::ComponentUpdater) {
        let mut queue = self.queue.lock().unwrap();
        for (height, callback) in queue.drain(..) {
            updater.terminal().insert_before(height, callback);
        }
    }
}

impl InsertBeforeHandler {
    pub fn insert_before<F>(&self, height: u16, callback: F)
    where
        F: FnOnce(&mut Buffer) + Send + 'static,
    {
        let mut queue = self.queue.lock().unwrap();
        queue.push_back((height, Box::new(callback)));
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
