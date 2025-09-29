//! hooks 模块：为组件提供响应式状态、副作用、事件、上下文等能力，灵感来源于 React Hooks。
//!
//! ## 如何实现一个规范的自定义 hook
//!
//! 1. 定义一个实现 [`Hook`] trait 的结构体，管理自己的状态和生命周期。
//! 2. 在 `poll_change`、`pre_component_update`、`post_component_update`、`pre_component_draw`、`post_component_draw` 等方法中实现副作用或状态逻辑。
//! 3. 提供 trait（如 `pub trait UseXxx`）暴露给用户，trait 方法通过 `Hooks::use_hook` 注册/获取 hook 实例。
//! 4. 推荐通过 `private::Sealed` 限制 trait 只对框架内部实现。
//!
//! ```rust
//! // 1. 定义 hook 状态结构体
//! pub struct MyHook { ... }
//! impl Hook for MyHook { ... }
//!
//! // 2. 提供 trait API
//! pub trait UseMyHook: private::Sealed {
//!     fn use_my_hook(&mut self, ...) -> ...;
//! }
//! impl UseMyHook for Hooks<'_, '_> {
//!     fn use_my_hook(&mut self, ...) -> ... {
//!         self.use_hook(|| MyHook { ... })
//!     }
//! }
//! ```
//!
//! 这样可保证 hook 生命周期、类型安全和复用性。

#![allow(unused)]
use crate::{
    context::ContextStack,
    render::{ComponentDrawer, ComponentUpdater},
};
use std::{
    any::Any,
    pin::Pin,
    task::{Context, Poll},
};
mod use_context;
pub use use_context::*;
mod use_events;
pub use use_events::*;
mod use_future;
pub use use_future::*;
mod use_state;
pub use use_state::*;
mod use_memo;
pub use use_memo::*;
mod use_effect;
pub use use_effect::*;
mod use_insert_before;
pub use use_insert_before::*;
mod use_size;
pub use use_size::*;
mod use_exit;
pub use use_exit::*;

#[cfg(feature = "router")]
mod use_router;
#[cfg(feature = "router")]
pub use use_router::*;

/// 所有自定义 hook 的 trait 基础，定义生命周期相关回调。
///
/// - `poll_change`：异步/响应式副作用轮询，适合 use_future/use_effect 等。
/// - `pre_component_update/post_component_update`：组件更新前后钩子。
/// - `pre_component_draw/post_component_draw`：组件渲染前后钩子。
///
/// 通常无需手动实现，除非自定义复杂 hook。
pub trait Hook: Unpin + Send {
    fn poll_change(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<()> {
        Poll::Pending
    }

    fn pre_component_update(&mut self, _updater: &mut ComponentUpdater) {}
    fn post_component_update(&mut self, _updater: &mut ComponentUpdater) {}

    fn pre_component_draw(&mut self, _drawer: &mut ComponentDrawer) {}
    fn post_component_draw(&mut self, _drawer: &mut ComponentDrawer) {}
}

pub(crate) trait AnyHook: Hook {
    fn any_self_mut(&mut self) -> &mut dyn Any;
}

impl<T: Hook + 'static> AnyHook for T {
    fn any_self_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl Hook for Vec<Box<dyn AnyHook>> {
    fn poll_change(mut self: Pin<&mut Self>, _cx: &mut Context) -> Poll<()> {
        let mut is_ready = false;
        for hook in self.iter_mut() {
            if Pin::new(&mut **hook).poll_change(_cx).is_ready() {
                is_ready = true;
            }
        }

        if is_ready {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }

    fn pre_component_update(&mut self, _updater: &mut ComponentUpdater) {
        for hook in self.iter_mut() {
            hook.pre_component_update(_updater);
        }
    }

    fn post_component_update(&mut self, _updater: &mut ComponentUpdater) {
        for hook in self.iter_mut() {
            hook.post_component_update(_updater);
        }
    }

    fn pre_component_draw(&mut self, _updater: &mut ComponentDrawer) {
        for hook in self.iter_mut() {
            hook.pre_component_draw(_updater);
        }
    }

    fn post_component_draw(&mut self, _updater: &mut ComponentDrawer) {
        for hook in self.iter_mut() {
            hook.post_component_draw(_updater);
        }
    }
}

/// hooks 管理器，负责组件内所有 hook 的注册、索引和生命周期。
///
/// - 通过 `use_hook` 注册/获取 hook 实例，保证顺序和类型安全。
/// - 支持 context 注入、首次更新标记等。
/// - 用户无需手动创建，框架自动管理。
///
/// # 示例
/// ```rust
/// let mut state = hooks.use_state(|| 0);
/// let ctx = hooks.use_context::<MyType>();
/// ```
pub struct Hooks<'a, 'b: 'a> {
    hooks: &'a mut Vec<Box<dyn AnyHook>>,
    first_update: bool,
    hook_index: usize,
    pub(crate) context: Option<&'a ContextStack<'b>>,
}

impl<'a> Hooks<'a, '_> {
    pub(crate) fn new(hooks: &'a mut Vec<Box<dyn AnyHook>>, first_update: bool) -> Self {
        Self {
            hooks,
            first_update,
            hook_index: 0,
            context: None,
        }
    }

    pub fn with_context_stack<'c, 'd>(
        &'c mut self,
        context: &'c ContextStack<'d>,
    ) -> Hooks<'c, 'd> {
        Hooks {
            hooks: self.hooks,
            first_update: self.first_update,
            hook_index: self.hook_index,
            context: Some(context),
        }
    }

    pub fn use_hook<F, H>(&mut self, f: F) -> &mut H
    where
        F: FnOnce() -> H,
        H: Hook + Unpin + 'static,
    {
        if self.first_update {
            self.hooks.push(Box::new(f()));
        }
        let idx = self.hook_index;
        self.hook_index += 1;

        self.hooks
            .get_mut(idx)
            .and_then(|hook| hook.any_self_mut().downcast_mut::<H>())
            .expect("Hook type mismatch, ensure the hook is of the correct type")
    }
}
