use generational_box::{
    AnyStorage, BorrowError, BorrowMutError, GenerationalBox, Owner, SyncStorage,
};
use std::collections::HashMap;
use std::sync::LazyLock;
use std::{
    cmp,
    fmt::{self, Debug, Display, Formatter},
    hash::{Hash, Hasher},
    ops::{self, Deref, DerefMut},
    task::Waker,
};

use crate::ElementKey;

mod use_store;
pub use use_store::UseStore;

static OWNER: LazyLock<Owner<SyncStorage>> = LazyLock::new(Owner::default);

struct StoreValue<T> {
    value: T,
    is_changed: bool,
    wakers: HashMap<ElementKey, Waker>,
}

pub struct StoreState<T>
where
    T: Send + Sync + 'static,
{
    inner: GenerationalBox<StoreValue<T>, SyncStorage>,
}

impl<T> StoreState<T>
where
    T: Send + Sync + 'static,
{
    pub fn new(value: T) -> Self {
        StoreState {
            inner: OWNER.insert(StoreValue {
                value,
                is_changed: false,
                wakers: HashMap::new(),
            }),
        }
    }
}

pub struct StoreStateRef<'a, T>
where
    T: 'static,
{
    inner: <SyncStorage as AnyStorage>::Ref<'a, StoreValue<T>>,
}

impl<T> Deref for StoreStateRef<'_, T>
where
    T: 'static,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner.value
    }
}

pub struct StoreStateMut<'a, T>
where
    T: 'static,
{
    inner: <SyncStorage as AnyStorage>::Mut<'a, StoreValue<T>>,
    is_deref_mut: bool,
}

impl<T> Deref for StoreStateMut<'_, T>
where
    T: 'static,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner.value
    }
}

impl<T> DerefMut for StoreStateMut<'_, T>
where
    T: 'static,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.is_deref_mut = true;
        &mut self.inner.value
    }
}

impl<T> Drop for StoreStateMut<'_, T>
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

impl<T> StoreState<T>
where
    T: Send + Sync + 'static,
{
    pub fn try_read(&self) -> Option<StoreStateRef<T>> {
        loop {
            match self.inner.try_read() {
                Ok(inner) => return Some(StoreStateRef { inner }),
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

    pub fn read(&self) -> StoreStateRef<T> {
        self.try_read()
            .expect("attempt to read state after owner was dropped")
    }

    pub fn try_write(&self) -> Option<StoreStateMut<T>> {
        self.inner
            .try_write()
            .map(|inner| StoreStateMut {
                inner,
                is_deref_mut: false,
            })
            .ok()
    }

    pub fn write(&self) -> StoreStateMut<T> {
        self.try_write()
            .expect("attempt to write state after owner was dropped")
    }

    pub fn set(&mut self, value: T) {
        if let Some(mut v) = self.try_write() {
            *v = value;
        }
    }
}

impl<T: Send + Sync + 'static> Clone for StoreState<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Send + Sync + 'static> Copy for StoreState<T> {}

impl<T: Send + Sync + Copy + 'static> StoreState<T> {
    pub fn get(&self) -> T {
        *self.read()
    }
}

impl<T: Debug + Sync + Send + 'static> Debug for StoreState<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.read().fmt(f)
    }
}

impl<T: Display + Sync + Send + 'static> Display for StoreState<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.read().fmt(f)
    }
}

impl<T: ops::Add<Output = T> + Copy + Sync + Send + 'static> ops::Add<T> for StoreState<T> {
    type Output = T;

    fn add(self, rhs: T) -> Self::Output {
        self.get() + rhs
    }
}

impl<T: ops::AddAssign<T> + Copy + Sync + Send + 'static> ops::AddAssign<T> for StoreState<T> {
    fn add_assign(&mut self, rhs: T) {
        if let Some(mut v) = self.try_write() {
            *v += rhs;
        }
    }
}

impl<T: ops::Sub<Output = T> + Copy + Sync + Send + 'static> ops::Sub<T> for StoreState<T> {
    type Output = T;

    fn sub(self, rhs: T) -> Self::Output {
        self.get() - rhs
    }
}

impl<T: ops::SubAssign<T> + Copy + Sync + Send + 'static> ops::SubAssign<T> for StoreState<T> {
    fn sub_assign(&mut self, rhs: T) {
        if let Some(mut v) = self.try_write() {
            *v -= rhs;
        }
    }
}

impl<T: ops::Mul<Output = T> + Copy + Sync + Send + 'static> ops::Mul<T> for StoreState<T> {
    type Output = T;

    fn mul(self, rhs: T) -> Self::Output {
        self.get() * rhs
    }
}

impl<T: ops::MulAssign<T> + Copy + Sync + Send + 'static> ops::MulAssign<T> for StoreState<T> {
    fn mul_assign(&mut self, rhs: T) {
        if let Some(mut v) = self.try_write() {
            *v *= rhs;
        }
    }
}

impl<T: ops::Div<Output = T> + Copy + Sync + Send + 'static> ops::Div<T> for StoreState<T> {
    type Output = T;

    fn div(self, rhs: T) -> Self::Output {
        self.get() / rhs
    }
}

impl<T: ops::DivAssign<T> + Copy + Sync + Send + 'static> ops::DivAssign<T> for StoreState<T> {
    fn div_assign(&mut self, rhs: T) {
        if let Some(mut v) = self.try_write() {
            *v /= rhs;
        }
    }
}

impl<T: Hash + Sync + Send> Hash for StoreState<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.read().hash(state)
    }
}

impl<T: cmp::PartialEq<T> + Sync + Send + 'static> cmp::PartialEq<T> for StoreState<T> {
    fn eq(&self, other: &T) -> bool {
        *self.read() == *other
    }
}

impl<T: cmp::PartialOrd<T> + Sync + Send + 'static> cmp::PartialOrd<T> for StoreState<T> {
    fn partial_cmp(&self, other: &T) -> Option<cmp::Ordering> {
        self.read().partial_cmp(other)
    }
}

impl<T: cmp::PartialEq<T> + Sync + Send + 'static> cmp::PartialEq<StoreState<T>> for StoreState<T> {
    fn eq(&self, other: &StoreState<T>) -> bool {
        *self.read() == *other.read()
    }
}

impl<T: cmp::PartialOrd<T> + Sync + Send + 'static> cmp::PartialOrd<StoreState<T>>
    for StoreState<T>
{
    fn partial_cmp(&self, other: &StoreState<T>) -> Option<cmp::Ordering> {
        self.read().partial_cmp(&other.read())
    }
}

impl<T: cmp::Eq + Sync + Send + 'static> cmp::Eq for StoreState<T> {}
