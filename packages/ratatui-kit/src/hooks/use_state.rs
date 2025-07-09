use super::{Hook, Hooks};
use generational_box::{
    AnyStorage, BorrowError, BorrowMutError, GenerationalBox, Owner, SyncStorage,
};
use std::{
    cmp,
    fmt::{self, Debug, Display, Formatter},
    hash::{Hash, Hasher},
    ops::{self, Deref, DerefMut},
    task::{Poll, Waker},
};

mod private {
    pub trait Sealed {}
    impl Sealed for crate::hooks::Hooks<'_, '_> {}
}

pub trait UseState: private::Sealed {
    /// 创建响应式状态，适合计数器、输入框等本地状态。
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
    pub fn new(initial_value: T) -> Self {
        let storage = Owner::default();
        UseStateImpl {
            state: State {
                inner: storage.insert(StateValue {
                    value: initial_value,
                    waker: None,
                    is_changed: false,
                }),
            },
            _storage: storage,
        }
    }
}

impl<T> Hook for UseStateImpl<T>
where
    T: Unpin + Send + Sync + 'static,
{
    fn poll_change(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context,
    ) -> std::task::Poll<()> {
        if let Ok(mut value) = self.state.inner.try_write() {
            if value.is_changed {
                value.is_changed = false;
                Poll::Ready(())
            } else {
                value.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        } else {
            Poll::Pending
        }
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

struct StateValue<T> {
    value: T,
    waker: Option<Waker>,
    is_changed: bool,
}

pub struct StateRef<'a, T: 'static> {
    inner: <SyncStorage as AnyStorage>::Ref<'a, StateValue<T>>,
}

impl<T: 'static> Deref for StateRef<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner.value
    }
}

pub struct StateMutRef<'a, T: 'static> {
    inner: <SyncStorage as AnyStorage>::Mut<'a, StateValue<T>>,
    is_deref_mut: bool,
}

impl<T: 'static> Deref for StateMutRef<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner.value
    }
}

impl<T: 'static> DerefMut for StateMutRef<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.is_deref_mut = true;
        &mut self.inner.value
    }
}

impl<T: 'static> Drop for StateMutRef<'_, T> {
    fn drop(&mut self) {
        if self.is_deref_mut {
            self.inner.is_changed = true;
            if let Some(waker) = self.inner.waker.take() {
                waker.wake();
            }
        }
    }
}

pub struct State<T: Send + Sync + 'static> {
    inner: GenerationalBox<StateValue<T>, SyncStorage>,
}

impl<T: Send + Sync + 'static> Clone for State<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Send + Sync + 'static> Copy for State<T> {}

impl<T: Send + Sync + Copy + 'static> State<T> {
    pub fn get(&self) -> T {
        *self.read()
    }
}

impl<T: Send + Sync + 'static> State<T> {
    pub fn try_read(&self) -> Option<StateRef<T>> {
        loop {
            match self.inner.try_read() {
                Ok(inner) => return Some(StateRef { inner }),
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

    pub fn read(&self) -> StateRef<T> {
        self.try_read()
            .expect("attempt to read state after owner was dropped")
    }

    pub fn try_write(&self) -> Option<StateMutRef<T>> {
        self.inner
            .try_write()
            .map(|inner| StateMutRef {
                inner,
                is_deref_mut: false,
            })
            .ok()
    }

    pub fn write(&self) -> StateMutRef<T> {
        self.try_write()
            .expect("attempt to write state after owner was dropped")
    }

    pub fn set(&mut self, value: T) {
        if let Some(mut v) = self.try_write() {
            *v = value;
        }
    }
}

impl<T: Debug + Sync + Send + 'static> Debug for State<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.read().fmt(f)
    }
}

impl<T: Display + Sync + Send + 'static> Display for State<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.read().fmt(f)
    }
}

impl<T: ops::Add<Output = T> + Copy + Sync + Send + 'static> ops::Add<T> for State<T> {
    type Output = T;

    fn add(self, rhs: T) -> Self::Output {
        self.get() + rhs
    }
}

impl<T: ops::AddAssign<T> + Copy + Sync + Send + 'static> ops::AddAssign<T> for State<T> {
    fn add_assign(&mut self, rhs: T) {
        if let Some(mut v) = self.try_write() {
            *v += rhs;
        }
    }
}

impl<T: ops::Sub<Output = T> + Copy + Sync + Send + 'static> ops::Sub<T> for State<T> {
    type Output = T;

    fn sub(self, rhs: T) -> Self::Output {
        self.get() - rhs
    }
}

impl<T: ops::SubAssign<T> + Copy + Sync + Send + 'static> ops::SubAssign<T> for State<T> {
    fn sub_assign(&mut self, rhs: T) {
        if let Some(mut v) = self.try_write() {
            *v -= rhs;
        }
    }
}

impl<T: ops::Mul<Output = T> + Copy + Sync + Send + 'static> ops::Mul<T> for State<T> {
    type Output = T;

    fn mul(self, rhs: T) -> Self::Output {
        self.get() * rhs
    }
}

impl<T: ops::MulAssign<T> + Copy + Sync + Send + 'static> ops::MulAssign<T> for State<T> {
    fn mul_assign(&mut self, rhs: T) {
        if let Some(mut v) = self.try_write() {
            *v *= rhs;
        }
    }
}

impl<T: ops::Div<Output = T> + Copy + Sync + Send + 'static> ops::Div<T> for State<T> {
    type Output = T;

    fn div(self, rhs: T) -> Self::Output {
        self.get() / rhs
    }
}

impl<T: ops::DivAssign<T> + Copy + Sync + Send + 'static> ops::DivAssign<T> for State<T> {
    fn div_assign(&mut self, rhs: T) {
        if let Some(mut v) = self.try_write() {
            *v /= rhs;
        }
    }
}

impl<T: Hash + Sync + Send> Hash for State<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.read().hash(state)
    }
}

impl<T: cmp::PartialEq<T> + Sync + Send + 'static> cmp::PartialEq<T> for State<T> {
    fn eq(&self, other: &T) -> bool {
        *self.read() == *other
    }
}

impl<T: cmp::PartialOrd<T> + Sync + Send + 'static> cmp::PartialOrd<T> for State<T> {
    fn partial_cmp(&self, other: &T) -> Option<cmp::Ordering> {
        self.read().partial_cmp(other)
    }
}

impl<T: cmp::PartialEq<T> + Sync + Send + 'static> cmp::PartialEq<State<T>> for State<T> {
    fn eq(&self, other: &State<T>) -> bool {
        *self.read() == *other.read()
    }
}

impl<T: cmp::PartialOrd<T> + Sync + Send + 'static> cmp::PartialOrd<State<T>> for State<T> {
    fn partial_cmp(&self, other: &State<T>) -> Option<cmp::Ordering> {
        self.read().partial_cmp(&other.read())
    }
}

impl<T: cmp::Eq + Sync + Send + 'static> cmp::Eq for State<T> {}
