use crate::{Atom, AtomState, ElementKey, Hook};
use std::task::Poll;

mod private {
    pub trait Sealed {}
    impl Sealed for crate::Hooks<'_, '_> {}
}

pub trait UseAtom: private::Sealed {
    /// 在组件内订阅一个全局原子：注册本组件的 waker，返回 `Copy + Send` 句柄
    /// （可移入 `tokio::spawn` 在后台更新）。写入仅唤醒订阅了该 atom 的组件（细粒度）。
    fn use_atom<T>(&mut self, atom: &'static Atom<T>) -> AtomState<T>
    where
        T: Unpin + Send + Sync + 'static;
}

impl UseAtom for crate::Hooks<'_, '_> {
    fn use_atom<T>(&mut self, atom: &'static Atom<T>) -> AtomState<T>
    where
        T: Unpin + Send + Sync + 'static,
    {
        let state = atom.state();
        let hook = self.use_hook(|| UseAtomImpl { state, key: None });
        hook.set_state(state);
        hook.state
    }
}

struct UseAtomImpl<T>
where
    T: Unpin + Send + Sync + 'static,
{
    state: AtomState<T>,
    key: Option<ElementKey>,
}

impl<T> UseAtomImpl<T>
where
    T: Unpin + Send + Sync + 'static,
{
    fn set_state(&mut self, state: AtomState<T>) {
        if self.state.same_storage(&state) {
            return;
        }

        if let Some(key) = &self.key {
            self.state.remove_waker(key);
        }
        self.state = state;
    }
}

impl<T> Hook for UseAtomImpl<T>
where
    T: Unpin + Send + Sync + 'static,
{
    fn poll_change(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context) -> Poll<()> {
        let Some(key) = self.key.clone() else {
            return Poll::Pending;
        };

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

    fn on_drop(&mut self) {
        if let Some(key) = &self.key {
            self.state.remove_waker(key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::task::noop_waker;
    use std::task::Context;

    /// 跑一次 `poll_change`（应 Pending），借此把组件 key 注册进 atom 的 waker 表。
    fn poll_once(hook: &mut UseAtomImpl<i32>) {
        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);
        assert!(std::pin::Pin::new(hook).poll_change(&mut cx).is_pending());
    }

    #[test]
    fn set_state_removes_old_subscription_when_atom_changes() {
        let old_state = AtomState::new(1i32);
        let new_state = AtomState::new(2i32);
        let key = ElementKey::decl(7);
        let mut hook = UseAtomImpl {
            state: old_state,
            key: Some(key.clone()),
        };

        poll_once(&mut hook);
        assert!(
            old_state
                .inner
                .try_read()
                .unwrap()
                .wakers
                .contains_key(&key)
        );

        hook.set_state(new_state);

        assert!(
            !old_state
                .inner
                .try_read()
                .unwrap()
                .wakers
                .contains_key(&key)
        );
        assert!(hook.state.same_storage(&new_state));
    }

    #[test]
    fn on_drop_removes_subscription() {
        let state = AtomState::new(1i32);
        let key = ElementKey::decl(8);
        let mut hook = UseAtomImpl {
            state,
            key: Some(key.clone()),
        };

        poll_once(&mut hook);
        assert!(state.inner.try_read().unwrap().wakers.contains_key(&key));

        hook.on_drop();

        assert!(!state.inner.try_read().unwrap().wakers.contains_key(&key));
    }
}
