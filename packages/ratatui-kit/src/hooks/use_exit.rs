use std::sync::Arc;

use crate::{SystemContext, UseContext, UseState};

mod private {
    pub trait Sealed {}
    impl Sealed for crate::hooks::Hooks<'_, '_> {}
}

pub trait UseExit: private::Sealed {
    /// 注册退出回调，组件卸载时调用，适合清理资源、保存状态等场景。
    fn use_exit(&mut self) -> impl FnMut() + Send + 'static;
}

impl UseExit for crate::hooks::Hooks<'_, '_> {
    fn use_exit(&mut self) -> impl FnMut() + Send + 'static {
        let mut state = self.use_state(|| false);
        let mut system_ctx = self.use_context_mut::<SystemContext>();

        if state.get() {
            system_ctx.exit();
        }

        move || {
            if !state.get() {
                state.set(true);
            }
        }
    }
}
