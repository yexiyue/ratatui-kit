---
title: "Hooks"
sidebar:
    order: 5
---

Ratatui Kit 内置了丰富的 Hooks 机制，帮助你在终端 UI 组件中高效管理状态、副作用和生命周期。Hooks 让组件逻辑更简洁、可复用，是响应式开发的核心能力。

## 什么是 Hooks？

Hooks 是一组函数式 API，专为组件内部的本地状态管理、事件响应、数据订阅等场景设计。通过 Hooks，你可以像在 React 生态中一样，优雅地组织和复用组件逻辑。

## 常用 Hooks 一览

- `use_future`：创建与组件生命周期绑定的异步任务
- `use_state`：管理组件本地状态
- `use_events`：处理全局终端事件（如键盘、鼠标等）
- `use_local_events`：处理组件渲染区域内的本地事件
- `use_effect`：注册同步副作用
- `use_async_effect`：注册异步副作用
- `use_memo`：依赖缓存，避免重复计算
- `use_context`：访问上层 ContextProvider 提供的数据
- `use_insert_before`：在组件渲染窗口前插入内容，参考 ratatui 的 `Terminal::insert_before`

## 各类 Hooks 详解

### use_future

用于注册异步副作用任务，适合定时器、网络请求、异步轮询等场景。任务与组件生命周期绑定，组件销毁时自动清理。

```rust
pub trait UseFuture: private::Sealed {
    fn use_future<F>(&mut self, f: F)
    where
        F: Future<Output = ()> + Send + 'static;
}
```

**示例：**

```rust
hooks.use_future(async move {
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        state += 1;
    }
});
```

完整示例请参考 `examples/counter.rs`

### use_state

用于在组件中创建和管理本地响应式状态，适合计数器、输入框等场景。返回的 `State<T>` 支持多种读写方式，线程安全。

```rust
pub trait UseState: private::Sealed {
    fn use_state<T, F>(&mut self, init: F) -> State<T>
    where
        F: FnOnce() -> T,
        T: Unpin + Send + Sync + 'static;
}
```

- `get`：获取当前状态值（T 实现 Copy 时）
- `set`：设置新状态
- `read`/`try_read`：获取不可变引用
- `write`/`try_write`：获取可变引用

**示例：**

```rust
let mut state = hooks.use_state(|| 0);

hooks.use_future(async move {
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        state += 1;
    }
});
```

如需管理复杂类型，可用 `read`/`write` 方法：

```rust
let scroll_view_state = hooks.use_state(ScrollViewState::default);

hooks.use_local_events(move |event| {
    scroll_view_state.write().handle_event(&event);
});
```

完整示例请参考 `examples/scrollview.rs`

### use_events / use_local_events

分别用于注册全局事件监听器和仅作用于当前组件的本地事件监听器。

```rust
pub trait UseEvents: private::Sealed {
    fn use_events<F>(&mut self, f: F)
    where
        F: FnMut(Event) + Send + 'static;

    fn use_local_events<F>(&mut self, f: F)
    where
        F: FnMut(Event) + Send + 'static;
}
```

- `use_events`：处理全局终端事件（如快捷键、全局输入等）
- `use_local_events`：仅处理组件渲染区域内的事件，适合局部交互

**示例：**

```rust
let mut open = hooks.use_state(|| false);
hooks.use_events(move |event| {
    if let Event::Key(key_event) = event {
        if key_event.kind == KeyEventKind::Press && key_event.code == KeyCode::Tab {
            open.set(!open.get());
        }
    }
});
```

完整示例请参考 `examples/modal.rs`

### use_effect / use_async_effect

用于注册副作用逻辑，依赖变化时自动执行。支持同步和异步两种模式。

```rust
pub trait UseEffect: private::Sealed {
    fn use_effect<F, D>(&mut self, f: F, deps: D)
    where
        F: FnOnce(),
        D: Hash;

    fn use_async_effect<F, D>(&mut self, f: F, deps: D)
    where
        F: Future<Output = ()> + Send + 'static,
        D: Hash;
}
```

- `use_effect`：同步副作用，依赖变化时自动执行
- `use_async_effect`：异步副作用，依赖变化时自动清理并重启任务
- `deps`：依赖项，实现 Hash trait 即可

**示例：**

```rust
// 实时解析 JSON
hooks.use_effect(
    move || match serde_json::from_str::<serde_json::Value>(&json_text.read()) {
        Ok(val) => {
            let pretty = serde_json::to_string_pretty(&val).unwrap_or_default();
            formatted.set(pretty);
            error.set(String::new());
        }
        Err(e) => {
            formatted.set(String::new());
            error.set(e.to_string());
        }
    },
    [json_text.read().clone()],
);
```

完整示例请参考 `examples/modal.rs`

### use_memo

用于依赖缓存，只有依赖变化时才重新计算，适合性能优化。

```rust
pub trait UseMemo: private::Sealed {
    fn use_memo<F, D, T>(&mut self, f: F, deps: D) -> T
    where
        F: FnOnce() -> T,
        D: Hash,
        T: Clone + Send + Unpin + 'static;
}
```

**示例：**

```rust
let lines = hooks.use_memo(
    || {
        let content = fs::read_to_string("README.md")
            .unwrap_or_else(|_| "无法读取 README.md".to_string());
        content.lines().map(|l| l.to_string()).collect::<Vec<_>>()
    },
    (),
);
```

此例中，依赖为空元组 `()`，表示只在首次渲染时执行一次。

完整示例请参考 `examples/scrollview.rs`

### use_context

用于获取全局或局部上下文，常配合 `ContextProvider` 组件实现依赖注入。

```rust
pub trait UseContext<'a>: private::Sealed {
    fn use_context<T: Any>(&self) -> Ref<'a, T>;
    fn use_context_mut<T: Any>(&self) -> RefMut<'a, T>;
    fn try_use_context<T: Any>(&self) -> Option<Ref<'a, T>>;
    fn try_use_context_mut<T: Any>(&self) -> Option<RefMut<'a, T>>;
}
```

**Context 创建方式：**

```rust
impl<'a> Context<'a> {
    pub fn owned<T: Any + Send + Sync>(context: T) -> Self {
        Context::Owned(Box::new(context))
    }
    pub fn form_ref<T: Any + Send + Sync>(context: &'a T) -> Self {
        Context::Ref(context)
    }
    pub fn form_mut<T: Any + Send + Sync>(context: &'a mut T) -> Self {
        Context::Mut(context)
    }
}
```

**用法示例：**

父组件：

```rust
element!(
    ContextProvider(
        value: Context::owned(value),
    )
)
```

子组件：

```rust
let mut system_ctx = hooks.use_context_mut::<Value>();
```

### use_insert_before

用于在终端渲染窗口前插入内容，类似 ratatui 的 `Terminal::insert_before`。返回 `InsertBeforeHandler`，可灵活插入和渲染内容。

```rust
pub trait UseInsertBefore: private::Sealed {
    fn use_insert_before(&mut self) -> InsertBeforeHandler;
}
```

- `insert_before`：在渲染前插入内容
- `render_before`：渲染插入内容，提供便捷接口
- `finish`：调用后才会重新渲染组件，插入内容生效

**示例：**

```rust
let insert_before = hooks.use_insert_before();

hooks.use_events(move |event| {
    if let Event::Key(key_event) = event {
        if key_event.kind == KeyEventKind::Press {
            match key_event.code {
                KeyCode::Esc => {
                    should_exit.set(true);
                }
                KeyCode::Enter => {
                    if !value.read().is_empty() {
                        insert_before
                            .render_before(Line::from(format!("message: {value}")), 1)
                            .finish();
                        value.set(String::new());
                    }
                }
                _ => {}
            }
        }
    }
});
```

完整示例请参考 `examples/textarea.rs`

通过上述 Hooks，你可以灵活管理组件的状态、副作用和上下文，极大提升终端 UI 的开发效率与可维护性。建议结合实际场景，优先选用最贴合需求的 Hook。

---

## 创建自定义 Hook

除了内置的丰富 Hooks，Ratatui Kit 还支持你根据实际业务需求，灵活扩展属于自己的 Hook。自定义 Hook 主要有两种方式：

1. 基于已有 Hooks 组合实现（推荐，简单高效）
2. 完全自定义 Hook（适合有特殊生命周期或底层需求的场景）

无论哪种方式，建议都先定义一个私有模块，为 Hooks 提供类型约束，这样可以让你的扩展更安全、更规范：

```rust
mod private {
    pub trait Sealed {}
    impl Sealed for crate::hooks::Hooks<'_, '_> {}
}
```

接下来，定义一个公共 trait，暴露你自定义的 Hook API：

```rust
pub trait MyHooks: private::Sealed {
    fn my_hook(&mut self) -> String;
}
```

然后为 `Hooks` 实现该 trait。你可以在这里自由组合已有的 Hooks，封装出更贴合业务的逻辑。例如：

```rust
impl MyHooks for crate::hooks::Hooks<'_, '_> {
    fn my_hook(&mut self) -> String {
        // 组合已有 Hooks 实现自定义逻辑
        let state = self.use_state(|| String::new());
        self.use_events(move |event| {
            if let Event::Key(key_event) = event {
                if key_event.kind == KeyEventKind::Press && key_event.code == KeyCode::Enter {
                    state.set("Hello, World!".to_string());
                }
            }
        });
        state.read().clone()
    }
}
```

这样，你就可以像调用内置 Hook 一样，直接在组件里使用 `hooks.my_hook()`，让代码更简洁、更易复用。

---

如果你需要更底层的控制，比如实现一个拥有独立生命周期的异步任务 Hook，可以参考下面以 `use_future` 为例的完整自定义流程：

### 完全自定义 Hook 实现示例

1. 先定义 trait，描述 Hook 的 API：

```rust
pub trait UseFuture: private::Sealed {
    fn use_future<F>(&mut self, f: F)
    where
        F: Future<Output = ()> + Send + 'static;
}
```

2. 定义保存异步任务的结构体：

```rust
pub struct UseFutureImpl {
    f: Option<BoxFuture<'static, ()>>,
}
```

3. 实现 Hook trait，定义生命周期相关方法。`Hook` trait 允许你在组件更新、绘制等阶段插入自定义逻辑：

```rust
pub trait Hook: Unpin + Send {
    fn poll_change(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<()> {
        Poll::Pending
    }
    fn pre_component_update(&mut self, _updater: &mut ComponentUpdater) {}
    fn post_component_update(&mut self, _updater: &mut ComponentUpdater) {}
    fn pre_component_draw(&mut self, _drawer: &mut ComponentDrawer) {}
    fn post_component_draw(&mut self, _drawer: &mut ComponentDrawer) {}
}
```

4. 为 `UseFutureImpl` 实现 Hook trait，管理异步任务的生命周期：

```rust
impl Hook for UseFutureImpl {
    fn poll_change(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context,
    ) -> std::task::Poll<()> {
        if let Some(future) = self.f.as_mut() {
            if future.as_mut().poll(cx).is_ready() {
                self.f = None; // 清除已完成的 future
            }
        }
        Poll::Pending
    }
}
```

这样，异步任务完成后会自动清理，组件不会因为任务完成而强制刷新。

5. 给 `UseFutureImpl` 添加构造方法，方便实例化：

```rust
impl UseFutureImpl {
    pub fn new<F>(f: F) -> Self
    where
        F: Future<Output = ()> + Send + 'static,
    {
        UseFutureImpl {
            f: Some(Box::pin(f)),
        }
    }
}
```

6. 最后为 `Hooks` 实现 `UseFuture` trait，将自定义 Hook 挂载到 Hooks 上：

```rust
impl UseFuture for Hooks<'_, '_> {
    fn use_future<F>(&mut self, f: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.use_hook(move || UseFutureImpl::new(f));
    }
}
```

现在，你就可以在组件中通过 `hooks.use_future(async { ... })` 注册自定义异步任务了。

自定义 Hook 能极大提升代码复用性和可维护性。无论是简单的业务逻辑复用，还是复杂的生命周期管理，都可以通过这种方式优雅实现。如果你想深入了解更多实现细节，欢迎直接阅读 [Ratatui Kit 原理讲解](https://yexiyue.github.io/ratatui-kit-website/principle/01-%E5%89%8D%E8%A8%80/)！
