---
title: "路由"
sidebar:
    order: 6
---

Ratatui Kit 提供了强大的路由功能，帮助你在终端应用中实现灵活的页面导航和管理。无论是简单的单页面跳转，还是复杂的嵌套路由，都能轻松应对。下面为你详细介绍路由的用法和最佳实践。

## 定义路由表

你可以通过 `routes!` 宏声明应用的所有路由。每个路由都需要指定路径和对应的组件。

```rust
let routes = routes! {
    "/" => HomePage,
    "/counter" => CounterPage,
    "/markdown" => MarkdownReader,
    "/input" => InputPage,
};
```

如需为路由添加参数，直接在路径中使用 `:param` 语法：

```rust
let routes = routes! {
    "/user/:id" => UserProfile,
    "/post/:id" => PostDetail,
};
```

支持嵌套路由，只需在组件后加 `{}`，即可为某个页面声明子路由：

```rust
let routes = routes! {
    "/" => HomePage {
        "/settings" => SettingsPage,
        "/profile" => ProfilePage,
    }
}
```

在父组件（如 HomePage）中，需要通过 `Outlet` 组件渲染当前激活的子路由：

```rust
#[component]
fn HomePage(hooks: Hooks) -> impl Into<AnyElement<'static>> {
    // ...
    element!(
        View {
            Outlet
        }
    )
}
```

## 启用路由功能

要让应用支持路由导航，需要在根组件外包裹 `RouterProvider`。这样所有子组件都能访问到路由上下文。

```rust
#[component]
fn App(hooks: Hooks) -> impl Into<AnyElement<'static>> {
    element!(
        RouterProvider(
            routes: routes,
            index_path: "/",
        )
    )
}
```

`routes` 用于指定路由表，`index_path` 是应用的默认首页路径。

## 路由导航

在组件中，你可以通过 `hooks.use_navigate()` 获取路由跳转器 `Navigate`，实现页面跳转、返回等操作。`Navigate` 提供了丰富的方法：

- `push`：跳转到新页面（入栈）
- `push_with_state`：跳转并携带状态
- `replace`：替换当前页面
- `replace_with_state`：替换并携带状态
- `go`：跳转到指定索引的页面
- `back`：返回上一个页面
- `forward`：前进到下一个页面

**示例：**

```rust
let mut navigate = hooks.use_navigate();

hooks.use_events(move |event| {
    if let Event::Key(key_event) = event {
        if key_event.kind == KeyEventKind::Press {
            match key_event.code {
                KeyCode::Char('1') => navigate.push("/counter"),
                KeyCode::Char('2') => navigate.push("/markdown"),
                KeyCode::Char('3') => navigate.push("/input"),
                _ => {}
            }
        }
    }
});
```

## 路由相关 Hooks

Ratatui Kit 还内置了多种路由相关 Hook，方便你在组件中获取路由信息、参数和状态：

```rust
pub trait UseRouter<'a>: private::Sealed {
    /// 获取路由跳转器，可用于页面跳转、返回等。
    fn use_navigate(&mut self) -> Navigate;
    /// 获取当前路由状态，适合页面间状态传递。
    fn use_route_state<T: Send + Sync + 'static>(&self) -> Option<Arc<T>>;
    /// 获取当前路由信息。
    fn use_route(&self) -> Ref<'a, Route>;
    /// 获取当前路由的可变引用。
    fn use_route_mut(&mut self) -> RefMut<'a, Route>;
    /// 获取当前路由参数。
    fn use_params(&self) -> Ref<'a, HashMap<String, String>>;
}
```

- `use_navigate`：获取路由跳转器
- `use_route_state`：获取页面间传递的状态
- `use_route` / `use_route_mut`：获取当前路由信息（只读/可变）
- `use_params`：获取当前路由参数

这些 Hook 能让你在组件中灵活获取和操作路由相关数据，满足各种页面逻辑需求。

## 示例与更多资料

你可以参考[路由示例](https://yexiyue.github.io/ratatui-kit-website/example/router/)获取完整代码和更多用法。如果在实际开发中遇到问题，欢迎查阅文档或在社区交流你的经验！
