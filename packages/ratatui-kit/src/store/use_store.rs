use crate::{ElementKey, Hook, StoreState};
use std::task::Poll;

mod private {
    pub trait Sealed {}
    impl Sealed for crate::Hooks<'_, '_> {}
}

pub trait UseStore: private::Sealed {
    fn use_store<T>(&mut self, state: StoreState<T>) -> StoreState<T>
    where
        T: Unpin + Send + Sync + 'static;
}

impl UseStore for crate::Hooks<'_, '_> {
    fn use_store<T>(&mut self, state: StoreState<T>) -> StoreState<T>
    where
        T: Unpin + Send + Sync + 'static,
    {
        let hook = self.use_hook(|| UseStoreImpl { state, key: None });
        hook.state
    }
}

struct UseStoreImpl<T>
where
    T: Unpin + Send + Sync + 'static,
{
    state: StoreState<T>,
    key: Option<ElementKey>,
}

impl<T> Hook for UseStoreImpl<T>
where
    T: Unpin + Send + Sync + 'static,
{
    fn poll_change(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context) -> Poll<()> {
        let key = self.key.clone().unwrap();
        if let Ok(mut value) = self.state.inner.try_write() {
            if value.is_changed {
                value.is_changed = false;
                value.wakers.clear();

                return Poll::Ready(());
            } else {
                value.wakers.insert(key, cx.waker().clone());
            }
        }
        Poll::Pending
    }

    fn post_component_update(&mut self, updater: &mut crate::ComponentUpdater) {
        if self.key.is_none() {
            self.key = Some(updater.key().clone());
        }
    }
}
