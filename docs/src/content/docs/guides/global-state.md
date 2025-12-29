---
title: "全局状态"
sidebar:
    order: 7
---


Ratatui Kit 内置了全局状态管理机制，帮助你在终端应用中高效地共享和管理跨组件的状态。无论是计数器、表单输入，还是更复杂的全局数据，都能轻松实现响应式更新。下面为你详细介绍全局状态的用法和注意事项。

## 定义全局状态

定义全局状态非常简单，只需为你的结构体派生 `Store` 宏即可：

```rust
#[derive(Store, Default)]
pub struct CounterAndTextInput {
    pub count: i32,
    pub value: String,
}
```

这样会自动生成一个 `CounterAndTextInputStore` 类型和一个 static `COUNTER_AND_TEXT_INPUT_STORE` 实例，方便全局访问。

## 在组件中使用全局状态

如果只用一个全局状态，直接通过 `hooks.use_store` 绑定即可：

```rust
pub trait UseStore: private::Sealed {
    fn use_store<T>(&mut self, state: StoreState<T>) -> StoreState<T>
    where
        T: Unpin + Send + Sync + 'static;
}
```

`use_store` 接收并返回一个 `StoreState<T>`，会自动将状态与当前组件绑定，实现状态变更时自动刷新组件。

如需同时使用多个全局状态，可以用 `use_stores!` 宏，支持批量绑定：

```rust
let store = &COUNTER_AND_TEXT_INPUT_STORE;
let (count, value) = use_stores!(store.count, store.value);
```

这样你就可以在组件中像使用本地状态一样，方便地读写全局状态。

## 注意事项

- `StoreState` 和 `State` 的实现原理基本一致，区别在于全局状态需要与组件绑定，才能实现自动刷新。未绑定时可读写，但不会自动更新组件。
- 使用 `Store` 派生宏时，结构体不能有类型参数，且必须实现 `Default` trait（用于初始化 static store）。

## 示例与更多资料

你可以参考[全局状态示例](https://yexiyue.github.io/ratatui-kit-website/example/store/)获取完整代码和更多用法。如果在实际开发中遇到问题，欢迎查阅文档或在社区交流你的经验！
