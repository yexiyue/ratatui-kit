use std::{
    cmp,
    collections::HashMap,
    fmt::{self, Debug, Display, Formatter},
    hash::{Hash, Hasher},
    ops::{Deref, DerefMut},
    task::{Context, Poll, Waker},
};

use generational_box::{AnyStorage, GenerationalBox, Owner, SyncStorage};

use crate::ElementKey;

#[doc(hidden)]
pub trait Notifier: Default + Send + Sync + 'static {
    fn wake(&mut self);
    fn register(&mut self, key: Option<&ElementKey>, waker: Waker);
    fn clear(&mut self);
    fn remove(&mut self, _key: &ElementKey) {}
}

#[derive(Default)]
#[doc(hidden)]
pub struct SingleWaker {
    waker: Option<Waker>,
}

impl Notifier for SingleWaker {
    fn wake(&mut self) {
        if let Some(waker) = self.waker.take() {
            waker.wake();
        }
    }

    fn register(&mut self, _key: Option<&ElementKey>, waker: Waker) {
        self.waker = Some(waker);
    }

    fn clear(&mut self) {
        self.waker = None;
    }
}

#[derive(Default)]
#[doc(hidden)]
pub struct WakerMap {
    wakers: HashMap<ElementKey, Waker>,
}

impl Notifier for WakerMap {
    fn wake(&mut self) {
        for waker in self.wakers.values() {
            waker.wake_by_ref();
        }
    }

    fn register(&mut self, key: Option<&ElementKey>, waker: Waker) {
        if let Some(key) = key {
            self.wakers.insert(key.clone(), waker);
        }
    }

    fn clear(&mut self) {
        self.wakers.clear();
    }

    fn remove(&mut self, key: &ElementKey) {
        self.wakers.remove(key);
    }
}

#[doc(hidden)]
pub struct ReactiveValue<T, N> {
    value: T,
    notifier: N,
    is_changed: bool,
}

// 响应式状态核心句柄。
pub struct ReactiveHandle<T, N>
where
    T: Send + Sync + 'static,
    N: Notifier,
{
    pub(crate) inner: GenerationalBox<ReactiveValue<T, N>, SyncStorage>,
}

impl<T, N> Clone for ReactiveHandle<T, N>
where
    T: Send + Sync + 'static,
    N: Notifier,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<T, N> Copy for ReactiveHandle<T, N>
where
    T: Send + Sync + 'static,
    N: Notifier,
{
}

impl<T, N> ReactiveHandle<T, N>
where
    T: Send + Sync + 'static,
    N: Notifier,
{
    pub(crate) fn new_in(owner: &Owner<SyncStorage>, value: T) -> Self {
        Self {
            inner: owner.insert(ReactiveValue {
                value,
                notifier: N::default(),
                is_changed: false,
            }),
        }
    }

    // 仅 `use_atom`(atom 特性)用于参数同步/退订,故随 atom 特性门控,避免无特性时的 dead_code 警告。
    #[cfg(feature = "atom")]
    pub(crate) fn same_storage(&self, other: &Self) -> bool {
        self.inner.ptr_eq(&other.inner)
    }

    #[cfg(feature = "atom")]
    pub(crate) fn remove_waker(&self, key: &ElementKey) {
        if let Ok(mut value) = self.inner.try_write() {
            value.notifier.remove(key);
        }
    }

    #[cfg(test)]
    pub(crate) fn has_waker(&self, key: &ElementKey) -> bool
    where
        N: WakerLookup,
    {
        self.inner
            .try_read()
            .map(|value| value.notifier.has_waker(key))
            .unwrap_or(false)
    }

    pub(crate) fn poll_change(&self, key: Option<&ElementKey>, cx: &mut Context<'_>) -> Poll<()> {
        if let Ok(mut value) = self.inner.try_write() {
            if value.is_changed {
                value.is_changed = false;
                value.notifier.clear();
                Poll::Ready(())
            } else {
                value.notifier.register(key, cx.waker().clone());
                Poll::Pending
            }
        } else {
            Poll::Pending
        }
    }

    // 尝试获取只读引用，失败时返回 None。
    pub fn try_read(&'_ self) -> Option<ReactiveRef<'_, T, N>> {
        self.inner
            .try_read()
            .ok()
            .map(|inner| ReactiveRef { inner })
    }

    // 获取只读引用，失败时 panic。
    pub fn read(&'_ self) -> ReactiveRef<'_, T, N> {
        self.try_read()
            .expect("attempt to read state while unavailable or already mutably borrowed")
    }

    // 尝试获取可变引用，支持变更通知，失败时返回 None。
    pub fn try_write(&'_ self) -> Option<ReactiveMutRef<'_, T, N>> {
        self.inner
            .try_write()
            .map(|inner| ReactiveMutRef {
                inner,
                is_deref_mut: false,
            })
            .ok()
    }

    // 获取可变引用，支持变更通知，失败时 panic。
    pub fn write(&'_ self) -> ReactiveMutRef<'_, T, N> {
        self.try_write()
            .expect("attempt to write state while unavailable or already borrowed")
    }

    // 尝试获取可变引用，不触发变更通知，失败时返回 None。
    pub fn try_write_no_update(&'_ self) -> Option<ReactiveMutNoUpdate<'_, T, N>> {
        self.inner
            .try_write()
            .map(|inner| ReactiveMutNoUpdate { inner })
            .ok()
    }

    // 获取可变引用，不触发变更通知，失败时 panic。
    pub fn write_no_update(&'_ self) -> ReactiveMutNoUpdate<'_, T, N> {
        self.try_write_no_update()
            .expect("attempt to write state while unavailable or already borrowed")
    }

    // 设置状态值，触发变更通知。
    pub fn set(&mut self, value: T) {
        if let Some(mut current) = self.try_write() {
            *current = value;
        }
    }

    // 设置状态值，不触发变更通知。
    pub fn set_no_update(&mut self, value: T) {
        if let Some(mut current) = self.try_write_no_update() {
            *current = value;
        }
    }
}

#[cfg(test)]
pub(crate) trait WakerLookup {
    fn has_waker(&self, key: &ElementKey) -> bool;
}

#[cfg(test)]
impl WakerLookup for WakerMap {
    fn has_waker(&self, key: &ElementKey) -> bool {
        self.wakers.contains_key(key)
    }
}

impl<T, N> ReactiveHandle<T, N>
where
    T: Send + Sync + Copy + 'static,
    N: Notifier,
{
    pub fn get(&self) -> T {
        *self.read()
    }
}

#[cfg(feature = "atom")]
impl<T> ReactiveHandle<T, WakerMap>
where
    T: Send + Sync + 'static,
{
    pub fn new(value: T) -> Self {
        Self::new_in(&crate::atom::OWNER, value)
    }
}

// 状态的只读引用。
pub struct ReactiveRef<'a, T, N>
where
    T: 'static,
    N: Notifier,
{
    inner: <SyncStorage as AnyStorage>::Ref<'a, ReactiveValue<T, N>>,
}

impl<T, N> Deref for ReactiveRef<'_, T, N>
where
    T: 'static,
    N: Notifier,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner.value
    }
}

// 状态的可变引用，支持变更通知。
pub struct ReactiveMutRef<'a, T, N>
where
    T: 'static,
    N: Notifier,
{
    inner: <SyncStorage as AnyStorage>::Mut<'a, ReactiveValue<T, N>>,
    is_deref_mut: bool,
}

impl<T, N> Deref for ReactiveMutRef<'_, T, N>
where
    T: 'static,
    N: Notifier,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner.value
    }
}

impl<T, N> DerefMut for ReactiveMutRef<'_, T, N>
where
    T: 'static,
    N: Notifier,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.is_deref_mut = true;
        &mut self.inner.value
    }
}

impl<T, N> Drop for ReactiveMutRef<'_, T, N>
where
    T: 'static,
    N: Notifier,
{
    fn drop(&mut self) {
        if self.is_deref_mut {
            self.inner.is_changed = true;
            self.inner.notifier.wake();
        }
    }
}

// 状态的可变引用，不触发变更通知。
pub struct ReactiveMutNoUpdate<'a, T, N>
where
    T: 'static,
    N: Notifier,
{
    inner: <SyncStorage as AnyStorage>::Mut<'a, ReactiveValue<T, N>>,
}

impl<T, N> Deref for ReactiveMutNoUpdate<'_, T, N>
where
    T: 'static,
    N: Notifier,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner.value
    }
}

impl<T, N> DerefMut for ReactiveMutNoUpdate<'_, T, N>
where
    T: 'static,
    N: Notifier,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner.value
    }
}

impl<T, N> Debug for ReactiveHandle<T, N>
where
    T: Debug + Send + Sync + 'static,
    N: Notifier,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.read().fmt(f)
    }
}

impl<T, N> Display for ReactiveHandle<T, N>
where
    T: Display + Send + Sync + 'static,
    N: Notifier,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.read().fmt(f)
    }
}

impl<T, N> Hash for ReactiveHandle<T, N>
where
    T: Hash + Send + Sync + 'static,
    N: Notifier,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.read().hash(state)
    }
}

impl<T, N> cmp::PartialEq<T> for ReactiveHandle<T, N>
where
    T: cmp::PartialEq<T> + Send + Sync + 'static,
    N: Notifier,
{
    fn eq(&self, other: &T) -> bool {
        *self.read() == *other
    }
}

impl<T, N> cmp::PartialOrd<T> for ReactiveHandle<T, N>
where
    T: cmp::PartialOrd<T> + Send + Sync + 'static,
    N: Notifier,
{
    fn partial_cmp(&self, other: &T) -> Option<cmp::Ordering> {
        self.read().partial_cmp(other)
    }
}

impl<T, N> cmp::PartialEq<ReactiveHandle<T, N>> for ReactiveHandle<T, N>
where
    T: cmp::PartialEq<T> + Send + Sync + 'static,
    N: Notifier,
{
    fn eq(&self, other: &ReactiveHandle<T, N>) -> bool {
        *self.read() == *other.read()
    }
}

impl<T, N> cmp::PartialOrd<ReactiveHandle<T, N>> for ReactiveHandle<T, N>
where
    T: cmp::PartialOrd<T> + Send + Sync + 'static,
    N: Notifier,
{
    fn partial_cmp(&self, other: &ReactiveHandle<T, N>) -> Option<cmp::Ordering> {
        self.read().partial_cmp(&other.read())
    }
}

impl<T, N> cmp::Eq for ReactiveHandle<T, N>
where
    T: cmp::Eq + Send + Sync + 'static,
    N: Notifier,
{
}

impl<T, N> std::ops::Add<T> for ReactiveHandle<T, N>
where
    T: std::ops::Add<Output = T> + Copy + Send + Sync + 'static,
    N: Notifier,
{
    type Output = T;

    fn add(self, rhs: T) -> T {
        self.get() + rhs
    }
}

impl<T, N> std::ops::AddAssign<T> for ReactiveHandle<T, N>
where
    T: std::ops::AddAssign<T> + Copy + Send + Sync + 'static,
    N: Notifier,
{
    fn add_assign(&mut self, rhs: T) {
        if let Some(mut current) = self.try_write() {
            *current += rhs;
        }
    }
}

impl<T, N> std::ops::Sub<T> for ReactiveHandle<T, N>
where
    T: std::ops::Sub<Output = T> + Copy + Send + Sync + 'static,
    N: Notifier,
{
    type Output = T;

    fn sub(self, rhs: T) -> T {
        self.get() - rhs
    }
}

impl<T, N> std::ops::SubAssign<T> for ReactiveHandle<T, N>
where
    T: std::ops::SubAssign<T> + Copy + Send + Sync + 'static,
    N: Notifier,
{
    fn sub_assign(&mut self, rhs: T) {
        if let Some(mut current) = self.try_write() {
            *current -= rhs;
        }
    }
}

impl<T, N> std::ops::Mul<T> for ReactiveHandle<T, N>
where
    T: std::ops::Mul<Output = T> + Copy + Send + Sync + 'static,
    N: Notifier,
{
    type Output = T;

    fn mul(self, rhs: T) -> T {
        self.get() * rhs
    }
}

impl<T, N> std::ops::MulAssign<T> for ReactiveHandle<T, N>
where
    T: std::ops::MulAssign<T> + Copy + Send + Sync + 'static,
    N: Notifier,
{
    fn mul_assign(&mut self, rhs: T) {
        if let Some(mut current) = self.try_write() {
            *current *= rhs;
        }
    }
}

impl<T, N> std::ops::Div<T> for ReactiveHandle<T, N>
where
    T: std::ops::Div<Output = T> + Copy + Send + Sync + 'static,
    N: Notifier,
{
    type Output = T;

    fn div(self, rhs: T) -> T {
        self.get() / rhs
    }
}

impl<T, N> std::ops::DivAssign<T> for ReactiveHandle<T, N>
where
    T: std::ops::DivAssign<T> + Copy + Send + Sync + 'static,
    N: Notifier,
{
    fn div_assign(&mut self, rhs: T) {
        if let Some(mut current) = self.try_write() {
            *current /= rhs;
        }
    }
}
