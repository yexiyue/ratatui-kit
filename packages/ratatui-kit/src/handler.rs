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

pub struct Handler<'a, T>(bool, Box<dyn FnMut(T) + Send + Sync + 'a>);

impl<T> Handler<'_, T> {
    pub fn is_default(&self) -> bool {
        !self.0
    }

    pub fn take(&mut self) -> Self {
        core::mem::take(self)
    }
}

impl<'a, T> Default for Handler<'a, T> {
    fn default() -> Self {
        Self(false, Box::new(|_| {}))
    }
}

impl<'a, F, T> From<F> for Handler<'a, T>
where
    F: FnMut(T) + Send + Sync + 'a,
{
    fn from(f: F) -> Self {
        Self(false, Box::new(f))
    }
}

impl<'a, T> Deref for Handler<'a, T> {
    type Target = Box<dyn FnMut(T) + Send + Sync + 'a>;

    fn deref(&self) -> &Self::Target {
        &self.1
    }
}

impl<'a, T> DerefMut for Handler<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.1
    }
}
