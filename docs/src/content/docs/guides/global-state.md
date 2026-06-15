---
title: "全局状态"
sidebar:
    order: 7
---


Ratatui Kit 内置 Atom 全局原子，帮助你在终端应用中共享跨组件状态。Atom 是进程级状态：可以在组件内订阅，也可以在组件外或后台任务里直接读写。

## 定义全局状态

Atom 使用模块级 `static` 声明，不需要结构体或派生宏：

```rust
static COUNT: Atom<i32> = Atom::new(|| 0);
static VALUE: Atom<String> = Atom::new(String::new);
```

`Atom::new` 接收无捕获初始化函数，首次读取、写入或 `use_atom` 时才会惰性创建底层状态。

## 在组件中使用全局状态

在组件内用 `hooks.use_atom(&ATOM)` 订阅全局原子。返回的 `AtomState<T>` 是 `Copy` 句柄，可以像本地 `State<T>` 一样读写：

```rust
#[component]
fn Counter(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut count = hooks.use_atom(&COUNT);

    hooks.use_future(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            count += 1;
        }
    });

    element!(Text(text: format!("Counter: {}", count.get())))
}
```

写入 `AtomState` 会唤醒订阅了同一个 Atom 的组件，实现细粒度刷新。

## 组件外读写

组件外可以通过 `Atom::get` / `Atom::set` 直接操作全局状态：

```rust
COUNT.set(10);
let current = COUNT.get();
```

如果需要把句柄移入后台任务，也可以先取得 `AtomState`：

```rust
let mut count = COUNT.state();
tokio::spawn(async move {
    count += 1;
});
```

## 注意事项

- `Atom<T>` 适合跨组件、跨页面或后台任务共享的状态；组件私有状态仍优先用 `use_state`。
- `hooks.use_atom(&ATOM)` 会注册当前组件的 waker。未订阅时仍可读写，但不会主动刷新任何组件。
- `AtomState<T>` 与 `State<T>` 都支持 `+=`、`-=` 等运算符重载，写入会触发变更通知。

## 示例与更多资料

你可以参考[全局状态示例](https://yexiyue.github.io/ratatui-kit-website/example/store/)获取完整代码和更多用法。
