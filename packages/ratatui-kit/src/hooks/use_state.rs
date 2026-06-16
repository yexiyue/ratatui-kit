//! 响应式状态管理 Hook 实现。
//!
//! 本模块为 ratatui-kit 提供了类似 React useState 的响应式状态管理能力，适用于计数器、输入框等本地状态。

use std::task::Poll;

use generational_box::{Owner, SyncStorage};

use super::{Hook, Hooks};
use crate::{ReactiveHandle, ReactiveMutNoUpdate, ReactiveMutRef, ReactiveRef, SingleWaker};

mod private {
    pub trait Sealed {}
    impl Sealed for crate::hooks::Hooks<'_, '_> {}
}

/// 响应式状态持有者。
pub type State<T> = ReactiveHandle<T, SingleWaker>;
/// 状态的只读引用。
pub type StateRef<'a, T> = ReactiveRef<'a, T, SingleWaker>;
/// 状态的可变引用，支持变更通知。
pub type StateMutRef<'a, T> = ReactiveMutRef<'a, T, SingleWaker>;
/// 状态的可变引用，不触发变更通知。
pub type StateMutNoUpdate<'a, T> = ReactiveMutNoUpdate<'a, T, SingleWaker>;

pub trait UseState: private::Sealed {
    /// 为 [`Hooks`] 提供 use_state 方法，创建响应式状态。
    fn use_state<T, F>(&mut self, init: F) -> State<T>
    where
        F: FnOnce() -> T,
        T: Unpin + Send + Sync + 'static;
}

struct UseStateImpl<T>
where
    T: Unpin + Send + Sync + 'static,
{
    state: State<T>,
    _storage: Owner<SyncStorage>,
}

impl<T> UseStateImpl<T>
where
    T: Unpin + Send + Sync + 'static,
{
    /// use_state 的内部实现，持有状态和存储。
    pub fn new(initial_value: T) -> Self {
        let storage = Owner::default();
        UseStateImpl {
            state: State::new_in(&storage, initial_value),
            _storage: storage,
        }
    }
}

impl<T> Hook for UseStateImpl<T>
where
    T: Unpin + Send + Sync + 'static,
{
    fn poll_change(&mut self, cx: &mut std::task::Context) -> std::task::Poll<()> {
        self.state.poll_change(None, cx)
    }
}

impl UseState for Hooks<'_, '_> {
    fn use_state<T, F>(&mut self, init: F) -> State<T>
    where
        F: FnOnce() -> T,
        T: Unpin + Send + Sync + 'static,
    {
        self.use_hook(move || UseStateImpl::new(init())).state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 经 UseStateImpl 持有 Owner 保活,即可在单测里拿到一个可用的 State<T>。
    // holder 在作用域内存活 → 底层 generational-box 有效。

    #[test]
    fn add_and_sub_assign_mutate_value() {
        let holder = UseStateImpl::new(0i32);
        let mut state = holder.state;
        state += 5;
        assert_eq!(state.get(), 5);
        state -= 2;
        assert_eq!(state.get(), 3);
    }

    #[test]
    fn mul_assign_mutates_value() {
        let holder = UseStateImpl::new(3i32);
        let mut state = holder.state;
        state *= 4;
        assert_eq!(state.get(), 12);
    }

    #[test]
    fn set_overwrites_and_get_reads() {
        let holder = UseStateImpl::new(10i32);
        let mut state = holder.state;
        state.set(99);
        assert_eq!(state.get(), 99);
    }

    #[test]
    fn copy_handles_share_storage() {
        let holder = UseStateImpl::new(1i32);
        let mut state = holder.state;
        let state2 = state;
        state += 41;
        assert_eq!(state.get(), 42);
        assert_eq!(state2.get(), 42);
    }
}
