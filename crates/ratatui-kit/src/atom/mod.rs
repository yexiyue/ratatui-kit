use std::sync::{LazyLock, OnceLock};

use generational_box::{Owner, SyncStorage};

use crate::{ReactiveHandle, ReactiveMutRef, ReactiveRef, WakerMap};

mod use_atom;
pub use use_atom::UseAtom;

pub(crate) static OWNER: LazyLock<Owner<SyncStorage>> = LazyLock::new(Owner::default);

pub type AtomState<T> = ReactiveHandle<T, WakerMap>;
pub type AtomStateRef<'a, T> = ReactiveRef<'a, T, WakerMap>;
pub type AtomStateMut<'a, T> = ReactiveMutRef<'a, T, WakerMap>;

/// 全局响应式原子（类 Jotai/Recoil）。
///
/// 模块级声明 `static COUNT: Atom<i32> = Atom::new(|| 0);`，零宏零结构。底层 `AtomState`
/// 在首次 `use_atom`/读写时**惰性**创建（插入进程级全局 `OWNER`）——因 generational-box
/// 需运行时初始化，故以 `OnceLock` 承载。`Atom`/`AtomState` 仍 `Send + Sync`（全局静态需
/// `Sync`），正好支撑后台 `tokio::spawn` 移动句柄更新状态。
///
/// 在组件内用 [`crate::UseAtom::use_atom`] 订阅；组件外/后台任务可经 [`Atom::get`]/[`Atom::set`]
/// 或经 `use_atom` 返回的 `Copy + Send` 句柄直接读写。
pub struct Atom<T>
where
    T: Send + Sync + 'static,
{
    init: fn() -> T,
    cell: OnceLock<AtomState<T>>,
}

impl<T> Atom<T>
where
    T: Send + Sync + 'static,
{
    /// 以无捕获初始化器声明一个全局原子（`const fn`，可作 `static`）。
    pub const fn new(init: fn() -> T) -> Self {
        Self {
            init,
            cell: OnceLock::new(),
        }
    }

    /// 惰性解析底层句柄（首次调用时以 `init()` 创建并插入全局 OWNER）。
    pub fn state(&self) -> AtomState<T> {
        *self.cell.get_or_init(|| AtomState::new((self.init)()))
    }

    /// 组件外直接读（`T: Copy`）。
    pub fn get(&self) -> T
    where
        T: Copy,
    {
        self.state().get()
    }

    /// 组件外直接写（触发订阅者重渲）。
    pub fn set(&self, value: T) {
        self.state().set(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // AtomState::new 用进程级全局 OWNER,无需额外保活即可在单测里使用。

    #[test]
    fn add_and_sub_assign_mutate_value() {
        let mut state = AtomState::new(0i32);
        state += 5;
        assert_eq!(state.get(), 5);
        state -= 2;
        assert_eq!(state.get(), 3);
    }

    #[test]
    fn set_overwrites_and_get_reads() {
        let mut state = AtomState::new(10i32);
        state.set(42);
        assert_eq!(state.get(), 42);
    }

    #[test]
    fn copy_handles_share_storage() {
        let mut state = AtomState::new(1i32);
        let state2 = state;
        state += 41;
        assert_eq!(state.get(), 42);
        assert_eq!(state2.get(), 42);
    }

    static A: Atom<i32> = Atom::new(|| 7);

    #[test]
    fn atom_lazy_init_and_get_set() {
        assert_eq!(A.get(), 7);
        A.set(10);
        assert_eq!(A.get(), 10);
    }

    #[test]
    fn atom_state_handle_shares_value() {
        static B: Atom<i32> = Atom::new(|| 0);
        let mut handle = B.state();
        handle += 5;
        assert_eq!(B.get(), 5);
        assert_eq!(B.state().get(), 5);
    }
}
