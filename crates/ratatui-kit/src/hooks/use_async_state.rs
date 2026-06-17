use std::future::Future;

use crate::{Hooks, State, UseEffect, UseState};

mod private {
    pub trait Sealed {}
    impl Sealed for crate::Hooks<'_, '_> {}
}

// 异步数据加载的三态结果。
pub struct AsyncState<T, E>
where
    T: Unpin + Send + Sync + 'static,
    E: Unpin + Send + Sync + 'static,
{
    pub data: State<Option<T>>,
    pub loading: State<bool>,
    pub error: State<Option<E>>,
}

pub trait UseAsyncState: private::Sealed {
    // 依赖变化时运行异步任务，并维护 data/loading/error 三态。
    fn use_async_state<F, Fut, D, T, E>(&mut self, f: F, deps: D) -> AsyncState<T, E>
    where
        F: FnOnce() -> Fut + 'static,
        Fut: Future<Output = Result<T, E>> + 'static,
        D: PartialEq + Unpin + 'static,
        T: Unpin + Send + Sync + 'static,
        E: Unpin + Send + Sync + 'static;
}

impl UseAsyncState for Hooks<'_, '_> {
    fn use_async_state<F, Fut, D, T, E>(&mut self, f: F, deps: D) -> AsyncState<T, E>
    where
        F: FnOnce() -> Fut + 'static,
        Fut: Future<Output = Result<T, E>> + 'static,
        D: PartialEq + Unpin + 'static,
        T: Unpin + Send + Sync + 'static,
        E: Unpin + Send + Sync + 'static,
    {
        let mut data = self.use_state(|| None::<T>);
        let mut loading = self.use_state(|| false);
        let mut error = self.use_state(|| None::<E>);

        self.use_async_effect(
            async move {
                loading.set(true);
                error.set(None);

                match f().await {
                    Ok(value) => {
                        data.set(Some(value));
                    }
                    Err(err) => {
                        error.set(Some(err));
                    }
                }

                loading.set(false);
            },
            deps,
        );

        AsyncState {
            data,
            loading,
            error,
        }
    }
}
