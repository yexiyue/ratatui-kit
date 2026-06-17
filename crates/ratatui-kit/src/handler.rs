/// 通用事件处理器类型，封装 FnMut 回调闭包，支持动态替换和默认空实现。
///
/// - 可用于组件 props 的事件回调（如 on_change、on_click 等）。
/// - 支持通过 `Handler::from` 包装任意闭包。
/// - `is_default()` 判断是否为默认空实现。
/// - `take()` 获取并重置 handler。
/// - 实现 Deref/DerefMut，可直接调用闭包。
///
/// # 示例
/// ```rust
/// let mut handler = Handler::from(|val| println!("changed: {}", val));
/// handler("hello");
/// ```
use core::ops::{Deref, DerefMut};

pub struct Handler<'a, T, V = ()>(bool, Box<dyn FnMut(T) -> V + Send + Sync + 'a>);

impl<T, V> Handler<'_, T, V>
where
    V: Default,
{
    pub fn is_default(&self) -> bool {
        self.0
    }

    pub fn take(&mut self) -> Self {
        core::mem::take(self)
    }
}

impl<'a, T, V> Default for Handler<'a, T, V>
where
    V: Default,
{
    fn default() -> Self {
        Self(true, Box::new(|_| V::default()))
    }
}

impl<'a, F, T, V> From<F> for Handler<'a, T, V>
where
    F: FnMut(T) -> V + Send + Sync + 'a,
{
    fn from(f: F) -> Self {
        Self(false, Box::new(f))
    }
}

impl<'a, T, V> Deref for Handler<'a, T, V> {
    type Target = Box<dyn FnMut(T) -> V + Send + Sync + 'a>;

    fn deref(&self) -> &Self::Target {
        &self.1
    }
}

impl<'a, T, V> DerefMut for Handler<'a, T, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.1
    }
}
