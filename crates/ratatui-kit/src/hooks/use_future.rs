use std::task::Poll;

use futures::{FutureExt, future::LocalBoxFuture};

use super::{Hook, Hooks};

mod private {
    pub trait Sealed {}

    impl Sealed for crate::hooks::Hooks<'_, '_> {}
}

pub trait UseFuture: private::Sealed {
    // 注册异步副作用任务，适合定时器、网络请求、异步轮询等场景。
    fn use_future<F>(&mut self, f: F)
    where
        F: Future<Output = ()> + 'static;
}

pub struct UseFutureImpl {
    f: Option<LocalBoxFuture<'static, ()>>,
}

impl UseFutureImpl {
    pub fn new<F>(f: F) -> Self
    where
        F: Future<Output = ()> + 'static,
    {
        UseFutureImpl {
            f: Some(f.boxed_local()),
        }
    }
}

impl Hook for UseFutureImpl {
    fn poll_change(&mut self, cx: &mut std::task::Context) -> std::task::Poll<()> {
        if let Some(future) = self.f.as_mut()
            && future.as_mut().poll(cx).is_ready()
        {
            self.f = None; // 清除已完成的 future
            return Poll::Ready(());
        }
        Poll::Pending
    }
}

impl UseFuture for Hooks<'_, '_> {
    fn use_future<F>(&mut self, f: F)
    where
        F: Future<Output = ()> + 'static,
    {
        self.use_hook(move || UseFutureImpl::new(f));
    }
}
