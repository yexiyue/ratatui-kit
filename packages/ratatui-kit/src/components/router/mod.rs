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

    /// 尝试把 `path` 匹配到本路由。
    ///
    /// 返回 `Some((剩余路径, 提取的命名参数))` 表示匹配,`None` 表示不匹配:
    /// - 动态路由(有 matcher):用预编译正则匹配前缀,并提取各 `:name` 参数;
    /// - 根路由 `"/"`:返回 `None`(留给 `Outlet` 最后兜底匹配);
    /// - 静态路由:前缀匹配且落在**段边界**(剩余为空或以 `/` 起始),否则不匹配
    ///   (避免 `"/book-source"` 误匹配 `"/book-source-login"`)。
    pub(crate) fn match_path(&self, path: &str) -> Option<(String, HashMap<String, String>)> {
        if let Some(regexp) = &self.matcher {
            let matched_len = regexp.find(path).map(|m| m.end()).unwrap_or(0);
            if matched_len == 0 {
                return None;
            }
            let mut params = HashMap::new();
            if let Some(caps) = regexp.captures(path) {
                for name in regexp.capture_names().flatten() {
                    if let Some(matched) = caps.name(name) {
                        params.insert(name.to_string(), matched.as_str().to_string());
                    }
                }
            }
            Some((path[matched_len..].to_string(), params))
        } else if self.path == "/" {
            None
        } else if path.starts_with(&self.path)
            && matches!(path[self.path.len()..].chars().next(), None | Some('/'))
        {
            Some((path[self.path.len()..].to_string(), HashMap::new()))
        } else {
            None
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::Fragment;

    // match_path 不关心组件,用任意 Fragment 元素占位。
    fn route(path: &str) -> Route {
        Route::new(
            path.to_string(),
            crate::element!(Fragment).into_any(),
            Routes::default(),
        )
    }

    #[test]
    fn dynamic_param_extracted_and_rest_empty() {
        let (rest, params) = route("/users/:id").match_path("/users/42").expect("应匹配");
        assert_eq!(params.get("id").map(String::as_str), Some("42"));
        assert_eq!(rest, "");
    }

    #[test]
    fn dynamic_segment_does_not_cross_slash() {
        let (rest, params) = route("/users/:id")
            .match_path("/users/42/profile")
            .expect("应匹配前缀");
        assert_eq!(params.get("id").map(String::as_str), Some("42"));
        assert_eq!(rest, "/profile");
    }

    #[test]
    fn static_match_respects_segment_boundary() {
        let r = route("/book-source");
        // 段中间不匹配:"/book-source-login" 的剩余 "-login" 不是新段。
        assert!(r.match_path("/book-source-login").is_none());
        // 精确匹配。
        assert_eq!(r.match_path("/book-source").unwrap().0, "");
        // 段边界(以 / 续)匹配。
        assert_eq!(r.match_path("/book-source/detail").unwrap().0, "/detail");
    }

    #[test]
    fn root_route_is_not_matched_here() {
        // "/" 留给 Outlet 兜底,不在 match_path 命中。
        assert!(route("/").match_path("/anything").is_none());
    }

    #[test]
    fn no_match_returns_none() {
        assert!(route("/settings").match_path("/profile").is_none());
    }
}
