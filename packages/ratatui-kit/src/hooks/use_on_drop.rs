use crate::Hook;

mod private {
    pub trait Sealed {}
    impl Sealed for crate::Hooks<'_, '_> {}
}

/// 在组件销毁时执行回调。注意不要在回调中使用State。
pub trait UseOnDrop: private::Sealed {
    fn use_on_drop<F>(&mut self, f: F)
    where
        F: FnMut() + Send + 'static;
}

#[derive(Default)]
struct UseOnDropImpl {
    callback: Option<Box<dyn FnMut() + Send>>,
}

impl Hook for UseOnDropImpl {
    fn on_drop(&mut self) {
        if let Some(mut callback) = self.callback.take() {
            callback();
        }
    }
}

impl UseOnDrop for crate::Hooks<'_, '_> {
    /// 在组件销毁时执行回调。注意不要在回调中使用State。
    fn use_on_drop<F>(&mut self, f: F)
    where
        F: FnMut() + Send + 'static,
    {
        let hook = self.use_hook(UseOnDropImpl::default);
        hook.callback.replace(Box::new(f));
    }
}
