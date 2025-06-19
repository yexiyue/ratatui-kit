use crate::AnyElement;
use std::{
    any::Any,
    collections::HashMap,
    ops::{Deref, DerefMut},
    sync::Arc,
};
mod outlet;
pub use outlet::*;
mod router_provider;
pub use router_provider::*;
pub(crate) mod history;

pub struct Route {
    pub path: String,
    pub component: AnyElement<'static>,
    pub children: Routes,
}

impl Route {
    pub fn borrow(&mut self) -> Route {
        Route {
            path: self.path.clone(),
            component: AnyElement::from(&mut self.component),
            children: self.children.borrow(),
        }
    }
}

unsafe impl Send for Route {}
unsafe impl Sync for Route {}

pub struct Routes(Vec<Route>);

impl Default for Routes {
    fn default() -> Self {
        Routes(Vec::new())
    }
}

impl Routes {
    pub fn borrow(&mut self) -> Routes {
        Routes(self.0.iter_mut().map(|r| r.borrow()).collect())
    }
}

impl From<Vec<Route>> for Routes {
    fn from(routes: Vec<Route>) -> Self {
        Routes(routes)
    }
}

impl Deref for Routes {
    type Target = Vec<Route>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Routes {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

unsafe impl Send for Routes {}
unsafe impl Sync for Routes {}

#[derive(Default, Clone)]
pub(crate) struct RouteContext {
    pub path: String,
    pub params: HashMap<String, String>,
    pub state: Option<Arc<dyn Any + Send + Sync>>,
}
