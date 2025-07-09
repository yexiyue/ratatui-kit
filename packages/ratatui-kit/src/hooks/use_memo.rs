use std::hash::{DefaultHasher, Hash, Hasher};

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
        D: Hash,
        T: Clone + Send + Unpin + 'static;
}

pub(crate) fn hash_deps<D: Hash>(deps: D) -> u64 {
    let mut hasher = DefaultHasher::new();
    deps.hash(&mut hasher);
    hasher.finish()
}

pub struct UseMemoImpl<T> {
    memoized_value: Option<T>,
    deps_hash: u64,
}

impl<T> Default for UseMemoImpl<T> {
    fn default() -> Self {
        UseMemoImpl {
            memoized_value: None,
            deps_hash: 0,
        }
    }
}

impl<T: Send + Unpin> Hook for UseMemoImpl<T> {}

impl UseMemo for Hooks<'_, '_> {
    fn use_memo<F, D, T>(&mut self, f: F, deps: D) -> T
    where
        F: FnOnce() -> T,
        D: Hash,
        T: Clone + Send + Unpin + 'static,
    {
        let dep_hash = hash_deps(deps);
        let hook = self.use_hook(UseMemoImpl::<T>::default);
        if hook.deps_hash != dep_hash || hook.memoized_value.is_none() {
            hook.memoized_value = Some(f());
            hook.deps_hash = dep_hash;
        }
        hook.memoized_value.clone().unwrap()
    }
}
