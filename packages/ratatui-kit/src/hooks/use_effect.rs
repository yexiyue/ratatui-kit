use futures::{FutureExt, future::BoxFuture};
use std::{hash::Hash, task::Poll};

use crate::{Hook, UseMemo, hash_deps};

mod private {
    pub trait Sealed {}
    impl Sealed for crate::Hooks<'_, '_> {}
}

pub trait UseEffect: private::Sealed {
    fn use_effect<F, D>(&mut self, f: F, deps: D)
    where
        F: FnOnce(),
        D: Hash;

    fn use_async_effect<F, D>(&mut self, f: F, deps: D)
    where
        F: Future<Output = ()> + Send + 'static,
        D: Hash;
}

#[derive(Default)]
pub struct UseAsyncEffectImpl {
    f: Option<BoxFuture<'static, ()>>,
    deps_hash: u64,
}

impl Hook for UseAsyncEffectImpl {
    fn poll_change(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context,
    ) -> std::task::Poll<()> {
        if let Some(future) = self.f.as_mut() {
            if future.as_mut().poll(cx).is_ready() {
                self.f = None;
            }
        }
        Poll::Pending
    }
}

impl UseEffect for crate::Hooks<'_, '_> {
    fn use_effect<F, D>(&mut self, f: F, deps: D)
    where
        F: FnOnce(),
        D: Hash,
    {
        self.use_memo(f, deps)
    }

    fn use_async_effect<F, D>(&mut self, f: F, deps: D)
    where
        F: Future<Output = ()> + Send + 'static,
        D: Hash,
    {
        let dep_hash = hash_deps(deps);
        let hook = self.use_hook(UseAsyncEffectImpl::default);

        if hook.deps_hash != dep_hash {
            hook.f = Some(f.boxed());
            hook.deps_hash = dep_hash;
        }
    }
}
