use std::task::Poll;

use futures::future::BoxFuture;

use super::{Hook, Hooks};

mod private {
    pub trait Sealed {}

    impl Sealed for crate::hooks::Hooks<'_, '_> {}
}

pub trait UseFuture: private::Sealed {
    /// 注册异步副作用任务，适合定时器、网络请求、异步轮询等场景。
    fn use_future<F>(&mut self, f: F)
    where
        F: Future<Output = ()> + Send + 'static;
}

pub struct UseFutureImpl {
    f: Option<BoxFuture<'static, ()>>,
}

impl UseFutureImpl {
    pub fn new<F>(f: F) -> Self
    where
        F: Future<Output = ()> + Send + 'static,
    {
        UseFutureImpl {
            f: Some(Box::pin(f)),
        }
    }
}

impl Hook for UseFutureImpl {
    fn poll_change(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context,
    ) -> std::task::Poll<()> {
        if let Some(future) = self.f.as_mut()
            && future.as_mut().poll(cx).is_ready() {
                self.f = None; // 清除已完成的 future
            }
        Poll::Pending
    }
}

impl UseFuture for Hooks<'_, '_> {
    fn use_future<F>(&mut self, f: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.use_hook(move || UseFutureImpl::new(f));
    }
}
