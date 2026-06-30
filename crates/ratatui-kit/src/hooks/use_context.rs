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
            ContextLookup::AlreadyBorrowed => {
                let ty = type_name::<T>();
                panic!(
                    "context `{ty}` is already borrowed: a guard of this type is still alive in \
                     the current scope. Drop the existing context guard before borrowing it again."
                )
            }
            ContextLookup::NotFound => {
                let ty = type_name::<T>();
                panic!(
                    "context `{ty}` not found: `use_context` only searches ancestor \
                     `ContextProvider`s, so a context provided by a sibling or descendant \
                     component is not visible here. Render this component inside the matching \
                     `ContextProvider`'s subtree, or use `try_use_context` to get an `Option` \
                     instead of panicking."
                )
            }
        }
    }

    fn use_context_mut<T: Any>(&self) -> RefMut<'a, T> {
        let stack = self.context.expect("context not available");
        match stack.get_context_mut::<T>() {
            ContextLookup::Found(res) => res,
            ContextLookup::AlreadyBorrowed => {
                let ty = type_name::<T>();
                panic!(
                    "context `{ty}` is already borrowed: a guard of this type is still alive in \
                     the current scope. Drop the existing context guard before borrowing it again."
                )
            }
            ContextLookup::NotFound => {
                let ty = type_name::<T>();
                panic!(
                    "context `{ty}` not found: `use_context_mut` only searches ancestor \
                     `ContextProvider`s, so a context provided by a sibling or descendant \
                     component is not visible here. Render this component inside the matching \
                     `ContextProvider`'s subtree, or use `try_use_context_mut` to get an `Option` \
                     instead of panicking."
                )
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::ContextStack;

    // 断言型 `use_context` 命中祖先注入的 context。
    #[test]
    fn use_context_returns_value_from_ancestor() {
        let mut hooks_vec = Vec::new();
        let mut hooks = Hooks::new(&mut hooks_vec, true);
        let mut root: i32 = 7;
        let stack = ContextStack::root(&mut root);
        let hooks = hooks.with_context_stack(&stack);
        assert_eq!(*hooks.use_context::<i32>(), 7);
    }

    // `try_use_context` 在缺失时安全降级为 `None`,绝不 panic。
    #[test]
    fn try_use_context_returns_none_when_absent() {
        let mut hooks_vec = Vec::new();
        let mut hooks = Hooks::new(&mut hooks_vec, true);
        let mut root: () = ();
        let stack = ContextStack::root(&mut root);
        let hooks = hooks.with_context_stack(&stack);
        assert!(hooks.try_use_context::<i32>().is_none());
    }

    // 断言型 `use_context` 缺失时 panic,且文案点明「只查祖先链」并指向 `try_use_context`。
    #[test]
    #[should_panic(expected = "only searches ancestor")]
    fn use_context_not_found_panics_with_helpful_message() {
        let mut hooks_vec = Vec::new();
        let mut hooks = Hooks::new(&mut hooks_vec, true);
        let mut root: () = ();
        let stack = ContextStack::root(&mut root);
        let hooks = hooks.with_context_stack(&stack);
        let _ = hooks.use_context::<i32>();
    }
}
