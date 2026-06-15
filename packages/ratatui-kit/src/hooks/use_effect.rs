use futures::{FutureExt, future::LocalBoxFuture};
use std::task::Poll;

use crate::{Hook, UseMemo};

mod private {
    pub trait Sealed {}
    impl Sealed for crate::Hooks<'_, '_> {}
}

pub trait UseEffect: private::Sealed {
    /// 注册同步副作用，依赖变化时自动执行，适合监听状态变化、同步校验等。
    fn use_effect<F, D>(&mut self, f: F, deps: D)
    where
        F: FnOnce(),
        D: PartialEq + Clone + Unpin + 'static;

    /// 注册异步副作用，依赖变化时自动执行，适合异步校验、异步请求等。
    fn use_async_effect<F, D>(&mut self, f: F, deps: D)
    where
        F: Future<Output = ()> + 'static,
        D: PartialEq + Clone + Unpin + 'static;
}

pub struct UseAsyncEffectImpl<D> {
    f: Option<LocalBoxFuture<'static, ()>>,
    deps: Option<D>,
}

impl<D> Default for UseAsyncEffectImpl<D> {
    fn default() -> Self {
        Self {
            f: None,
            deps: None,
        }
    }
}

impl<D: Unpin> Hook for UseAsyncEffectImpl<D> {
    fn poll_change(&mut self, cx: &mut std::task::Context) -> std::task::Poll<()> {
        if let Some(future) = self.f.as_mut()
            && future.as_mut().poll(cx).is_ready()
        {
            self.f = None;
            return Poll::Ready(());
        }
        Poll::Pending
    }
}

impl UseEffect for crate::Hooks<'_, '_> {
    fn use_effect<F, D>(&mut self, f: F, deps: D)
    where
        F: FnOnce(),
        D: PartialEq + Clone + Unpin + 'static,
    {
        self.use_memo(f, deps)
    }

    fn use_async_effect<F, D>(&mut self, f: F, deps: D)
    where
        F: Future<Output = ()> + 'static,
        D: PartialEq + Clone + Unpin + 'static,
    {
        let hook = self.use_hook(UseAsyncEffectImpl::<D>::default);

        if hook.deps.as_ref() != Some(&deps) {
            hook.f = Some(f.boxed_local());
            hook.deps = Some(deps.clone());
        }
    }
}
