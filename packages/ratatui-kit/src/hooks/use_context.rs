use std::{
    any::Any,
    cell::{Ref, RefMut},
};

use super::Hooks;

mod private {
    pub trait Sealed {}

    impl Sealed for crate::hooks::Hooks<'_, '_> {}
}

pub trait UseContext<'a>: private::Sealed {
    /// 获取全局/局部上下文，实现依赖注入。适合主题、配置、全局状态等场景。
    fn use_context<T: Any>(&self) -> Ref<'a, T>;
    /// 获取可变上下文。
    fn use_context_mut<T: Any>(&self) -> RefMut<'a, T>;
    /// 尝试获取只读上下文，返回 Option。
    fn try_use_context<T: Any>(&self) -> Option<Ref<'a, T>>;
    /// 尝试获取可变上下文，返回 Option。
    fn try_use_context_mut<T: Any>(&self) -> Option<RefMut<'a, T>>;
}

impl<'a> UseContext<'a> for Hooks<'a, '_> {
    fn use_context<T: Any>(&self) -> Ref<'a, T> {
        self.context
            .expect("context not available")
            .get_context()
            .expect("context not found")
    }

    fn use_context_mut<T: Any>(&self) -> RefMut<'a, T> {
        self.context
            .expect("context not available")
            .get_context_mut()
            .expect("context not found")
    }

    fn try_use_context<T: Any>(&self) -> Option<Ref<'a, T>> {
        self.context
            .and_then(|context_stack| context_stack.get_context())
    }

    fn try_use_context_mut<T: Any>(&self) -> Option<RefMut<'a, T>> {
        self.context
            .and_then(|context_stack| context_stack.get_context_mut())
    }
}
