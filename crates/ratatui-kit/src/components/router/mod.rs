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
    // 含动态参数(`/:name`)的路由的匹配正则,在构造时一次性编译并以 `Arc` 共享,
    // 供 `Outlet` 每次渲染复用,避免重复编译(见 `outlet.rs`)。静态路由为 `None`。
    // 私有字段:故 `Route` 须经 `Route::new` 构造(`routes!` 宏即如此)。
    matcher: Option<Arc<regex::Regex>>,
}

impl Route {
    // 构造路由,并在此**一次性**编译动态参数匹配正则。
    //
    // 路径含 `/:name` 段时,将其转为命名捕获组 `(?<name>[^/]+)`(只匹配单段、不跨 `/`),
    // 编译为正则并以 `Arc` 持有;非法正则在此 panic(路径为 `routes!` 中的静态字面量,
    // 属开发期错误,构造期暴露优于每次渲染暴露)。静态路由不编译、不持有正则。
    //
    // 两个关键不变量,与静态路由的 `starts_with` 前缀语义保持一致:
    // - **锚定开头**:pattern 以 `^` 起始,只在路径**前缀**匹配。否则 `regexp.find` 会命中
    //   路径中段——`/users/:id` 误匹配 `/foo/users/42`。
    // - **静态段转义**:非 `:param` 段经 `regex::escape`,使 `.` `+` 等正则元字符按字面匹配。
    //   否则 `/v1.0/:id` 的 `.` 会变通配,误匹配 `/v1x0/42`。
    pub fn new(path: String, component: AnyElement<'static>, children: Routes) -> Self {
        let matcher = if path.contains("/:") {
            let pattern = path
                .split('/')
                .map(|seg| match seg.strip_prefix(':') {
                    Some(name) => format!("(?<{name}>[^/]+)"),
                    None => regex::escape(seg),
                })
                .collect::<Vec<_>>()
                .join("/");
            Some(Arc::new(
                regex::Regex::new(&format!("^{pattern}")).expect("Invalid route path regex"),
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

    // 尝试把 `path` 匹配到本路由。
    //
    // `Outlet` 按 `routes!` 声明顺序选择第一个匹配项；同前缀静态路由应声明在动态路由之前。
    //
    // 返回 `Some((剩余路径, 提取的命名参数))` 表示匹配,`None` 表示不匹配:
    // - 动态路由(有 matcher):用预编译正则匹配前缀,并提取各 `:name` 参数;
    // - 根路由 `"/"`:返回 `None`(留给 `Outlet` 最后兜底匹配);
    // - 静态路由:前缀匹配且落在**段边界**(剩余为空或以 `/` 起始),否则不匹配
    //   (避免 `"/book-source"` 误匹配 `"/book-source-login"`)。
    pub(crate) fn match_path(&self, path: &str) -> Option<(String, HashMap<String, String>)> {
        if let Some(regexp) = &self.matcher {
            let matched_len = regexp.find(path).map(|m| m.end()).unwrap_or(0);
            if matched_len == 0 {
                return None;
            }
            if !matches!(path[matched_len..].chars().next(), None | Some('/')) {
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
}

pub struct Routes(Vec<Route>);

#[allow(clippy::derivable_impls)]
impl Default for Routes {
    fn default() -> Self {
        Routes(Vec::new())
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

    #[test]
    fn multiple_dynamic_params_extracted() {
        let (rest, params) = route("/users/:uid/posts/:pid")
            .match_path("/users/7/posts/42")
            .expect("应匹配多参数");
        assert_eq!(params.get("uid").map(String::as_str), Some("7"));
        assert_eq!(params.get("pid").map(String::as_str), Some("42"));
        assert_eq!(rest, "");
    }

    #[test]
    fn dynamic_route_is_anchored_at_start() {
        // 回归:动态正则须锚定开头,不能命中路径中段
        // (未锚定时 `/users/:id` 会误匹配 `/foo/users/42`)。
        assert!(
            route("/users/:id").match_path("/foo/users/42").is_none(),
            "动态路由不应匹配中段"
        );
    }

    #[test]
    fn static_segment_in_dynamic_route_is_escaped() {
        // 回归:动态路由里的静态段须 regex::escape,`.` 不能当通配。
        let r = route("/v1.0/:id");
        let (rest, params) = r.match_path("/v1.0/9").expect("字面点应匹配");
        assert_eq!(params.get("id").map(String::as_str), Some("9"));
        assert_eq!(rest, "");
        assert!(r.match_path("/v1x0/9").is_none(), "点不应作通配");
    }

    #[test]
    fn dynamic_param_requires_nonempty_segment() {
        // `[^/]+` 要求至少一个字符:空参数段 / 缺参数段都不匹配。
        assert!(
            route("/users/:id").match_path("/users/").is_none(),
            "空参数段不应匹配"
        );
        assert!(
            route("/users/:id").match_path("/users").is_none(),
            "缺参数段不应匹配"
        );
    }

    #[test]
    fn dynamic_param_accepts_dots_and_dashes() {
        // 参数值可含非 `/` 的任意字符(点、连字符)。
        let (_, params) = route("/file/:name")
            .match_path("/file/a-b.txt")
            .expect("应匹配");
        assert_eq!(params.get("name").map(String::as_str), Some("a-b.txt"));
    }

    #[test]
    fn dynamic_route_no_prefix_match_is_none() {
        assert!(route("/users/:id").match_path("/posts/1").is_none());
    }

    #[test]
    fn dynamic_param_value_does_not_swallow_following_segment() {
        // 参数只吃单段,后续段留入 rest 供嵌套 Outlet 续匹配。
        let (rest, params) = route("/u/:id")
            .match_path("/u/5/detail")
            .expect("应匹配前缀");
        assert_eq!(params.get("id").map(String::as_str), Some("5"));
        assert_eq!(rest, "/detail");
    }

    #[test]
    fn dynamic_static_tail_respects_segment_boundary() {
        let route = route("/users/:id/edit");
        assert!(route.match_path("/users/42/edit-more").is_none());
        assert_eq!(route.match_path("/users/42/edit").unwrap().0, "");
        assert_eq!(route.match_path("/users/42/edit/sub").unwrap().0, "/sub");
    }

    #[test]
    fn static_trailing_slash_is_rest() {
        // 段边界:精确匹配剩余空,带尾斜杠剩余 "/"。
        assert_eq!(route("/a").match_path("/a").unwrap().0, "");
        assert_eq!(route("/a").match_path("/a/").unwrap().0, "/");
    }

    #[test]
    fn route_state_downcasts_to_correct_type() {
        let s = RouteState::new(42u32);
        assert_eq!(s.downcast::<u32>().map(|a| *a), Some(42));
    }

    #[test]
    fn route_state_downcast_wrong_type_is_none() {
        let s = RouteState::new(42u32);
        assert!(s.downcast::<String>().is_none());
    }

    // --- routes! 宏传 props(右侧复用 element! 的 `Comp(prop: val)` 语法)---
    use crate::components::Text;
    use ratatui_kit_macros::{Props, component};

    #[derive(Default, Props)]
    struct GreetProps {
        name: String,
    }

    #[component]
    fn Greet(props: &GreetProps) -> impl Into<crate::AnyElement<'static>> {
        crate::element!(Text(text: props.name.clone()))
    }

    #[test]
    fn routes_macro_accepts_props() {
        // 圆括号传 props:`Comp(prop: val)`。
        let routes: Vec<Route> = crate::routes! {
            "/hi" => Greet(name: "world".to_string()),
        };
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].path, "/hi");
    }

    #[test]
    fn routes_macro_accepts_props_with_children() {
        // 护栏:props 的 `)` 后 `{}` 干净交还子路由解析,二者不串扰。
        let routes: Vec<Route> = crate::routes! {
            "/a" => Greet(name: "x".to_string()) {
                "/b" => Greet(name: "y".to_string()),
            },
        };
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].path, "/a");
        assert_eq!(routes[0].children.len(), 1);
        assert_eq!(routes[0].children[0].path, "/b");
    }

    #[test]
    fn routes_macro_no_props_still_works() {
        // 回归:无 props 的裸组件写法仍成立(parse_head 的无括号路径)。
        let routes: Vec<Route> = crate::routes! {
            "/home" => Fragment,
        };
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].path, "/home");
    }
}
