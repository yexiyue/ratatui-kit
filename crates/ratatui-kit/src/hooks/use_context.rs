use std::{
    any::{Any, type_name},
    cell::{Ref, RefMut},
};

use super::Hooks;
use crate::context::ContextLookup;

mod private {
    pub trait Sealed {}

    impl Sealed for crate::hooks::Hooks<'_, '_> {}
}

pub trait UseContext<'a>: private::Sealed {
    // 获取全局/局部上下文，实现依赖注入。适合主题、配置、全局状态等场景。
    fn use_context<T: Any>(&self) -> Ref<'a, T>;
    // 获取可变上下文。
    fn use_context_mut<T: Any>(&self) -> RefMut<'a, T>;
    // 尝试获取只读上下文，返回 Option。
    fn try_use_context<T: Any>(&self) -> Option<Ref<'a, T>>;
    // 尝试获取可变上下文，返回 Option。
    fn try_use_context_mut<T: Any>(&self) -> Option<RefMut<'a, T>>;
}

impl<'a> UseContext<'a> for Hooks<'a, '_> {
    fn use_context<T: Any>(&self) -> Ref<'a, T> {
        let stack = self.context.expect("context not available");
        match stack.get_context::<T>() {
            ContextLookup::Found(res) => res,
            ContextLookup::AlreadyBorrowed => panic!(
                "context `{}` 已被借用，请先释放现有 context 守卫",
                type_name::<T>()
            ),
            ContextLookup::NotFound => panic!("context `{}` not found", type_name::<T>()),
        }
    }

    fn use_context_mut<T: Any>(&self) -> RefMut<'a, T> {
        let stack = self.context.expect("context not available");
        match stack.get_context_mut::<T>() {
            ContextLookup::Found(res) => res,
            ContextLookup::AlreadyBorrowed => panic!(
                "context `{}` 已被借用，请先释放现有 context 守卫",
                type_name::<T>()
            ),
            ContextLookup::NotFound => panic!("context `{}` not found", type_name::<T>()),
        }
    }

    fn try_use_context<T: Any>(&self) -> Option<Ref<'a, T>> {
        match self.context?.get_context::<T>() {
            ContextLookup::Found(res) => Some(res),
            _ => None,
        }
    }

    fn try_use_context_mut<T: Any>(&self) -> Option<RefMut<'a, T>> {
        match self.context?.get_context_mut::<T>() {
            ContextLookup::Found(res) => Some(res),
            _ => None,
        }
    }
}
