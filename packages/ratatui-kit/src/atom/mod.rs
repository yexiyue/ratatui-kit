use generational_box::{
    AnyStorage, BorrowError, BorrowMutError, GenerationalBox, Owner, SyncStorage,
};
use std::collections::HashMap;
use std::sync::{LazyLock, OnceLock};
use std::{
    cmp,
    fmt::{self, Debug, Display, Formatter},
    hash::{Hash, Hasher},
    ops::{Deref, DerefMut},
    task::Waker,
};

use crate::ElementKey;

mod use_atom;
pub use use_atom::UseAtom;

static OWNER: LazyLock<Owner<SyncStorage>> = LazyLock::new(Owner::default);

struct AtomValue<T> {
    value: T,
    is_changed: bool,
    wakers: HashMap<ElementKey, Waker>,
}

pub struct AtomState<T>
where
    T: Send + Sync + 'static,
{
    inner: GenerationalBox<AtomValue<T>, SyncStorage>,
}

impl<T> AtomState<T>
where
    T: Send + Sync + 'static,
{
    pub fn new(value: T) -> Self {
        AtomState {
            inner: OWNER.insert(AtomValue {
                value,
                is_changed: false,
                wakers: HashMap::new(),
            }),
        }
    }

    fn same_storage(&self, other: &Self) -> bool {
        self.inner.ptr_eq(&other.inner)
    }

    fn remove_waker(&self, key: &ElementKey) {
        if let Ok(mut value) = self.inner.try_write() {
            value.wakers.remove(key);
        }
    }
}

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

pub struct AtomStateRef<'a, T>
where
    T: 'static,
{
    inner: <SyncStorage as AnyStorage>::Ref<'a, AtomValue<T>>,
}

impl<T> Deref for AtomStateRef<'_, T>
where
    T: 'static,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner.value
    }
}

pub struct AtomStateMut<'a, T>
where
    T: 'static,
{
    inner: <SyncStorage as AnyStorage>::Mut<'a, AtomValue<T>>,
    is_deref_mut: bool,
}

impl<T> Deref for AtomStateMut<'_, T>
where
    T: 'static,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner.value
    }
}

impl<T> DerefMut for AtomStateMut<'_, T>
where
    T: 'static,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.is_deref_mut = true;
        &mut self.inner.value
    }
}

impl<T> Drop for AtomStateMut<'_, T>
where
    T: 'static,
{
    fn drop(&mut self) {
        if self.is_deref_mut {
            self.inner.is_changed = true;
            for waker in self.inner.wakers.values() {
                waker.wake_by_ref();
            }
        }
    }
}

impl<T> AtomState<T>
where
    T: Send + Sync + 'static,
{
    pub fn try_read(&'_ self) -> Option<AtomStateRef<'_, T>> {
        loop {
            match self.inner.try_read() {
                Ok(inner) => return Some(AtomStateRef { inner }),
                Err(BorrowError::Dropped(_)) => {
                    return None;
                }
                Err(BorrowError::AlreadyBorrowedMut(_)) => match self.inner.try_write() {
                    Err(BorrowMutError::Dropped(_)) => {
                        return None;
                    }
                    _ => continue,
                },
            }
        }
    }

    pub fn read(&'_ self) -> AtomStateRef<'_, T> {
        self.try_read()
            .expect("attempt to read state after owner was dropped")
    }

    pub fn try_write(&'_ self) -> Option<AtomStateMut<'_, T>> {
        self.inner
            .try_write()
            .map(|inner| AtomStateMut {
                inner,
                is_deref_mut: false,
            })
            .ok()
    }

    pub fn write(&'_ self) -> AtomStateMut<'_, T> {
        self.try_write()
            .expect("attempt to write state after owner was dropped")
    }

    pub fn set(&mut self, value: T) {
        if let Some(mut v) = self.try_write() {
            *v = value;
        }
    }
}

impl<T: Send + Sync + 'static> Clone for AtomState<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Send + Sync + 'static> Copy for AtomState<T> {}

impl<T: Send + Sync + Copy + 'static> AtomState<T> {
    pub fn get(&self) -> T {
        *self.read()
    }
}

impl<T: Debug + Sync + Send + 'static> Debug for AtomState<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.read().fmt(f)
    }
}

impl<T: Display + Sync + Send + 'static> Display for AtomState<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.read().fmt(f)
    }
}

// 算术运算符重载：与 State 同构，由单一宏生成（见 reactive_ops.rs）。
crate::reactive_ops::impl_reactive_ops!(AtomState);

impl<T: Hash + Sync + Send> Hash for AtomState<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.read().hash(state)
    }
}

impl<T: cmp::PartialEq<T> + Sync + Send + 'static> cmp::PartialEq<T> for AtomState<T> {
    fn eq(&self, other: &T) -> bool {
        *self.read() == *other
    }
}

impl<T: cmp::PartialOrd<T> + Sync + Send + 'static> cmp::PartialOrd<T> for AtomState<T> {
    fn partial_cmp(&self, other: &T) -> Option<cmp::Ordering> {
        self.read().partial_cmp(other)
    }
}

impl<T: cmp::PartialEq<T> + Sync + Send + 'static> cmp::PartialEq<AtomState<T>> for AtomState<T> {
    fn eq(&self, other: &AtomState<T>) -> bool {
        *self.read() == *other.read()
    }
}

impl<T: cmp::PartialOrd<T> + Sync + Send + 'static> cmp::PartialOrd<AtomState<T>> for AtomState<T> {
    fn partial_cmp(&self, other: &AtomState<T>) -> Option<cmp::Ordering> {
        self.read().partial_cmp(&other.read())
    }
}

impl<T: cmp::Eq + Sync + Send + 'static> cmp::Eq for AtomState<T> {}

#[cfg(test)]
mod tests {
    use super::*;

    // AtomState::new 用进程级全局 OWNER,无需额外保活即可在单测里使用。

    #[test]
    fn add_and_sub_assign_mutate_value() {
        let mut s = AtomState::new(0i32);
        s += 5;
        assert_eq!(s.get(), 5);
        s -= 2;
        assert_eq!(s.get(), 3);
    }

    #[test]
    fn set_overwrites_and_get_reads() {
        let mut s = AtomState::new(10i32);
        s.set(42);
        assert_eq!(s.get(), 42);
    }

    #[test]
    fn copy_handles_share_storage() {
        let mut s = AtomState::new(1i32);
        let s2 = s; // Copy:同一底层 box
        s += 41;
        assert_eq!(s.get(), 42);
        assert_eq!(s2.get(), 42);
    }

    // ---- Atom（全局原子声明）----

    static A: Atom<i32> = Atom::new(|| 7);

    #[test]
    fn atom_lazy_init_and_get_set() {
        // 首次访问才以 init() 初始化。
        assert_eq!(A.get(), 7);
        A.set(10);
        assert_eq!(A.get(), 10);
    }

    #[test]
    fn atom_state_handle_shares_value() {
        static B: Atom<i32> = Atom::new(|| 0);
        // 多次 state()/get 指向同一底层 box（同一 atom）。
        let mut h = B.state();
        h += 5;
        assert_eq!(B.get(), 5);
        assert_eq!(B.state().get(), 5);
    }
}
