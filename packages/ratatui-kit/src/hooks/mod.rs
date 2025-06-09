#![allow(unused)]
use crate::{
    context::ContextStack,
    render::{ComponentDrawer, ComponentUpdater},
};
use std::{
    any::Any,
    pin::Pin,
    task::{Context, Poll},
};
mod use_context;
pub use use_context::UseContext;
mod use_events;
pub use use_events::UseEvents;
mod use_future;
pub use use_future::UseFuture;
mod use_state;
pub use use_state::UseState;

pub trait Hook: Unpin + Send {
    fn poll_change(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<()> {
        Poll::Pending
    }

    fn pre_component_update(&mut self, _updater: &mut ComponentUpdater) {}
    fn post_component_update(&mut self, _updater: &mut ComponentUpdater) {}

    fn pre_component_draw(&mut self, _drawer: &mut ComponentDrawer) {}
    fn post_component_draw(&mut self, _drawer: &mut ComponentDrawer) {}
}

pub(crate) trait AnyHook: Hook {
    fn any_self_mut(&mut self) -> &mut dyn Any;
}

impl<T: Hook + 'static> AnyHook for T {
    fn any_self_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl Hook for Vec<Box<dyn AnyHook>> {
    fn poll_change(mut self: Pin<&mut Self>, _cx: &mut Context) -> Poll<()> {
        let mut is_ready = false;
        for hook in self.iter_mut() {
            if Pin::new(&mut **hook).poll_change(_cx).is_ready() {
                is_ready = true;
            }
        }

        if is_ready {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }

    fn pre_component_update(&mut self, _updater: &mut ComponentUpdater) {
        for hook in self.iter_mut() {
            hook.pre_component_update(_updater);
        }
    }

    fn post_component_update(&mut self, _updater: &mut ComponentUpdater) {
        for hook in self.iter_mut() {
            hook.post_component_update(_updater);
        }
    }

    fn pre_component_draw(&mut self, _updater: &mut ComponentDrawer) {
        for hook in self.iter_mut() {
            hook.pre_component_draw(_updater);
        }
    }

    fn post_component_draw(&mut self, _updater: &mut ComponentDrawer) {
        for hook in self.iter_mut() {
            hook.post_component_draw(_updater);
        }
    }
}

pub struct Hooks<'a, 'b: 'a> {
    hooks: &'a mut Vec<Box<dyn AnyHook>>,
    first_update: bool,
    hook_index: usize,
    pub(crate) context: Option<&'a ContextStack<'b>>,
}

impl<'a> Hooks<'a, '_> {
    pub(crate) fn new(hooks: &'a mut Vec<Box<dyn AnyHook>>, first_update: bool) -> Self {
        Self {
            hooks,
            first_update,
            hook_index: 0,
            context: None,
        }
    }

    pub fn with_context_stack<'c, 'd>(
        &'c mut self,
        context: &'c ContextStack<'d>,
    ) -> Hooks<'c, 'd> {
        Hooks {
            hooks: self.hooks,
            first_update: self.first_update,
            hook_index: self.hook_index,
            context: Some(context),
        }
    }

    pub fn use_hook<F, H>(&mut self, f: F) -> &mut H
    where
        F: FnOnce() -> H,
        H: Hook + Unpin + 'static,
    {
        if self.first_update {
            self.hooks.push(Box::new(f()));
        }
        let idx = self.hook_index;
        self.hook_index += 1;

        self.hooks
            .get_mut(idx)
            .and_then(|hook| hook.any_self_mut().downcast_mut::<H>())
            .expect("Hook type mismatch, ensure the hook is of the correct type")
    }
}
