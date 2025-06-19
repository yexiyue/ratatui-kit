use std::{
    cell::{Ref, RefMut},
    collections::HashMap,
    sync::Arc,
};

use crate::{
    Handler, State, UseContext,
    prelude::{Route, RouteContext, history::RouterHistory, split_path},
};

mod private {
    pub trait Sealed {}
    impl Sealed for crate::Hooks<'_, '_> {}
}

pub trait UseRouter<'a>: private::Sealed {
    fn use_navigate(&mut self) -> Navigate;

    fn use_route_state<T: Send + Sync + 'static>(&self) -> Option<Arc<T>>;

    fn use_route(&self) -> Ref<'a, Route>;

    fn use_route_mut(&mut self) -> RefMut<'a, Route>;

    fn use_params(&self) -> Ref<'a, HashMap<String, String>>;
}

impl<'a> UseRouter<'a> for crate::Hooks<'a, '_> {
    fn use_navigate(&mut self) -> Navigate {
        let history = self.use_context::<State<RouterHistory>>();
        Navigate::new(*history)
    }

    fn use_route_state<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        let route_context = self.use_context::<RouteContext>();

        route_context
            .state
            .as_ref()
            .cloned()
            .and_then(|p| p.downcast::<T>().ok())
    }

    fn use_route(&self) -> Ref<'a, Route> {
        self.use_context::<Route>()
    }

    fn use_route_mut(&mut self) -> RefMut<'a, Route> {
        self.use_context_mut::<Route>()
    }

    fn use_params(&self) -> Ref<'a, HashMap<String, String>> {
        let ctx = self.use_context::<RouteContext>();
        Ref::map(ctx, |c| &c.params)
    }
}

#[derive(Clone, Copy)]
pub struct Navigate {
    history: State<RouterHistory>,
}

impl Navigate {
    pub(crate) fn new(history: State<RouterHistory>) -> Self {
        Navigate { history }
    }

    pub fn push(&mut self, path: String) {
        let mut history = self.history.write();
        let mut ctx = history.current_context();
        ctx.path = split_path(&path);
        history.push(ctx);
    }

    pub fn push_with_state<T>(&mut self, path: String, state: T)
    where
        T: Send + Sync + 'static,
    {
        let mut history = self.history.write();
        let mut ctx = history.current_context();
        ctx.path = split_path(&path);
        ctx.state = Some(Arc::new(state));
        history.push(ctx);
    }

    pub fn replace(&mut self, path: String) {
        let mut history = self.history.write();
        let mut ctx = history.current_context();
        ctx.path = split_path(&path);
        history.replace(ctx);
    }

    pub fn replace_with_state<T>(&mut self, path: String, state: T)
    where
        T: Send + Sync + 'static,
    {
        let mut history = self.history.write();
        let mut ctx = history.current_context();
        ctx.path = split_path(&path);
        ctx.state = Some(Arc::new(state));
        history.replace(ctx);
    }

    pub fn go(&mut self, delta: i32) {
        let mut history = self.history.write();
        history.go(delta);
    }

    pub fn back(&mut self) {
        let mut history = self.history.write();
        history.back();
    }

    pub fn forward(&mut self) {
        let mut history = self.history.write();
        history.forward();
    }
}
