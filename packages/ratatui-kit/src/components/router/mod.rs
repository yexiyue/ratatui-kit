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
    /// 含动态参数(`/:name`)的路由的匹配正则,在构造时一次性编译并以 `Arc` 共享,
    /// 供 `Outlet` 每次渲染复用,避免重复编译(见 `outlet.rs`)。静态路由为 `None`。
    /// 私有字段:故 `Route` 须经 `Route::new` 构造(`routes!` 宏即如此),
    /// crate 内的 `borrow()` 透传同一 `Arc`。
    matcher: Option<Arc<regex::Regex>>,
}

impl Route {
    /// 构造路由,并在此**一次性**编译动态参数匹配正则。
    ///
    /// 路径含 `/:name` 段时,将其转为命名捕获组 `(?<name>[^/]+)`(只匹配单段、不跨 `/`),
    /// 编译为正则并以 `Arc` 持有;非法正则在此 panic(路径为 `routes!` 中的静态字面量,
    /// 属开发期错误,构造期暴露优于每次渲染暴露)。静态路由不编译、不持有正则。
    pub fn new(path: String, component: AnyElement<'static>, children: Routes) -> Self {
        let matcher = if path.contains("/:") {
            let pattern = path
                .split('/')
                .map(|seg| match seg.strip_prefix(':') {
                    Some(name) => format!("(?<{name}>[^/]+)"),
                    None => seg.to_string(),
                })
                .collect::<Vec<_>>()
                .join("/");
            Some(Arc::new(
                regex::Regex::new(&pattern).expect("Invalid route path regex"),
            ))
        } else {
            None
        };

        Route {
            path,
            component,
            children,
            matcher,
        }
    }

    /// 本路由的预编译匹配正则(动态路由为 `Some`,静态路由为 `None`)。
    pub(crate) fn matcher(&self) -> Option<&Arc<regex::Regex>> {
        self.matcher.as_ref()
    }

    pub fn borrow(&mut self) -> Route {
        Route {
            path: self.path.clone(),
            component: AnyElement::from(&mut self.component),
            children: self.children.borrow(),
            // 透传同一已编译正则(Arc 共享),不重新编译。
            matcher: self.matcher.clone(),
        }
    }
}

unsafe impl Send for Route {}
unsafe impl Sync for Route {}

pub struct Routes(Vec<Route>);

#[allow(clippy::derivable_impls)]
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
    pub state: Option<RouteState>,
}

#[derive(Debug, Clone)]
pub struct RouteState(pub Arc<dyn Any + Send + Sync>);

impl RouteState {
    pub fn new<T>(state: T) -> Self
    where
        T: Any + Send + Sync + 'static,
    {
        RouteState(Arc::new(state))
    }

    pub fn downcast<T>(&self) -> Option<Arc<T>>
    where
        T: Any + Send + Sync + 'static,
    {
        self.0.clone().downcast().ok()
    }
}
