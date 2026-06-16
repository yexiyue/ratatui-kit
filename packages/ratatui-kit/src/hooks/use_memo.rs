use crate::{Hook, Hooks};

mod private {
    pub trait Sealed {}
    impl Sealed for crate::Hooks<'_, '_> {}
}

pub trait UseMemo: private::Sealed {
    /// 依赖缓存，只有依赖变化时才重新计算，适合性能优化。
    fn use_memo<F, D, T>(&mut self, f: F, deps: D) -> T
    where
        F: FnOnce() -> T,
        D: PartialEq + Unpin + 'static,
        T: Clone + Unpin + 'static;
}

pub struct UseMemoImpl<T, D> {
    memoized_value: Option<T>,
    deps: Option<D>,
}

impl<T, D> Default for UseMemoImpl<T, D> {
    fn default() -> Self {
        Self {
            memoized_value: None,
            deps: None,
        }
    }
}

impl<T: Unpin, D: Unpin> Hook for UseMemoImpl<T, D> {}

impl UseMemo for Hooks<'_, '_> {
    fn use_memo<F, D, T>(&mut self, f: F, deps: D) -> T
    where
        F: FnOnce() -> T,
        D: PartialEq + Unpin + 'static,
        T: Clone + Unpin + 'static,
    {
        let hook = self.use_hook(UseMemoImpl::<T, D>::default);
        if hook.deps.as_ref() != Some(&deps) || hook.memoized_value.is_none() {
            hook.memoized_value = Some(f());
            hook.deps = Some(deps);
        }
        hook.memoized_value.clone().expect("memoized value is set")
    }
}
