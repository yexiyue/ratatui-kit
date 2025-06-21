use crate::{
    AnyElement, Context, Hooks, UseContext,
    prelude::{ContextProvider, RouteContext, Routes},
};
use ratatui_kit_macros::{component, element};

// 定义 `Outlet` 组件，用于实现路由嵌套和路径匹配逻辑。
// 该组件会根据当前路径匹配一个子路由，并更新上下文中的路径为未匹配部分。
#[component]
pub fn Outlet<'a>(hooks: Hooks) -> impl Into<AnyElement<'a>> {
    // 获取全局路由表和当前路径上下文
    let mut routes = hooks.use_context_mut::<Routes>();
    let mut route_context = hooks.use_context_mut::<RouteContext>();

    // 查找与当前路径匹配的第一个路由
    let mut current_route = routes.iter_mut().find(|r| {
        let path = route_context.path.clone();

        // 判断路径是否包含动态参数（例如 "/users/:id"）
        if r.path.contains("/:") {
            // 将路径按 '/' 分割成多个段
            let regexp = r
                .path
                .split("/")
                .map(|s| {
                    // 如果是动态参数段（以 ':' 开头），则生成正则表达式捕获组
                    if s.starts_with(":") {
                        let name = s.trim_start_matches(":");
                        format!("(?<{}>[^/]+)", name) // 使用 [^/]+ 确保只匹配单个路径段
                    } else {
                        s.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("/"); // 合并所有段形成完整的正则表达式

            // 编译正则表达式
            let regexp = regex::Regex::new(&regexp).expect("Invalid route path");

            // 计算匹配长度
            let matched_len = regexp.find(&path).map(|m| m.end()).unwrap_or(0);

            // 如果没有匹配到，则返回 false 表示不匹配此路由
            if matched_len == 0 {
                return false;
            }

            // 提取动态参数并保存到 route_context.params 中
            if let Some(caps) = regexp.captures(&path) {
                for name in regexp.capture_names().flatten() {
                    if let Some(matched) = caps.name(name) {
                        route_context
                            .params
                            .insert(name.to_string(), matched.as_str().to_string());
                    }
                }
            }

            // 更新上下文中的路径为未匹配的部分
            route_context.path = path[matched_len..].to_string();
            true
        } else if r.path == "/" {
            // 如果路由路径是根路径 "/"，则不在此处处理（留给最后兜底匹配）
            false
        } else if path.starts_with(&r.path) {
            // 如果当前路径以静态路径开头，则更新上下文路径为剩余部分
            route_context.path = path[r.path.len()..].to_string();
            true
        } else {
            // 不满足任何条件，不匹配此路由
            false
        }
    });

    // 如果没有找到匹配的路由，则尝试匹配根路径 "/"
    if current_route.is_none() {
        current_route = routes.iter_mut().find(|r| r.path == "/");
    }

    // 解包 Option 并确保存在匹配的路由
    let current_route = current_route.expect("No matching route found");

    // 构建当前路由对应的 UI 元素
    let current_element = AnyElement::from(&mut current_route.component);

    // 返回构建的 UI 树结构
    element!(ContextProvider(
        value: Context::owned(current_route.children.borrow())
    ) {
        ContextProvider(
            value: Context::owned(current_route.borrow())
        ) {
            #(current_element)
        }
    })
}
