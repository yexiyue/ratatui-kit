use super::RouteContext;
use std::collections::VecDeque;

#[derive(Default, Clone)]
pub(crate) struct RouterHistory {
    pub current: usize,
    pub history: VecDeque<RouteContext>,
    pub max_length: usize,
}

impl RouterHistory {
    pub fn push(&mut self, context: RouteContext) {
        if self.history.len() >= self.max_length {
            self.history.pop_front();
        }
        self.history.push_back(context);
        self.current = self.history.len() - 1;
    }

    pub fn replace(&mut self, route: RouteContext) {
        self.history[self.current] = route;
    }

    pub fn back(&mut self) -> Option<RouteContext> {
        if self.current > 0 {
            self.current -= 1;
            Some(self.history[self.current].clone())
        } else {
            None
        }
    }

    pub fn forward(&mut self) -> Option<RouteContext> {
        if self.current < self.history.len() - 1 {
            self.current += 1;
            Some(self.history[self.current].clone())
        } else {
            None
        }
    }

    pub fn go(&mut self, n: i32) -> Option<RouteContext> {
        let new_index = self.current as i32 + n;

        if new_index >= 0 && (new_index as usize) < self.history.len() {
            self.current = new_index as usize;
            Some(self.history[self.current].clone())
        } else {
            None
        }
    }

    pub fn current_context(&self) -> RouteContext {
        self.history.get(self.current).unwrap().clone()
    }
}
