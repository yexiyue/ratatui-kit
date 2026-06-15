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
        // atom.state() 惰性解析底层句柄;use_hook 仅首帧执行此闭包,故只解析一次。
        let hook = self.use_hook(|| UseAtomImpl {
            state: atom.state(),
            key: None,
        });
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

impl<T> Hook for UseAtomImpl<T>
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
