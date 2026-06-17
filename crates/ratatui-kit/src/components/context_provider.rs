//! ContextProvider 组件：上下文依赖注入容器，为子组件提供全局/局部 context。
//!
//! 常用于全局状态、主题、配置等跨组件共享数据场景。
//!
//! ## 示例
//! ```rust
//! element!(ContextProvider(value: Some(Context::owned(MyData { ... }))) {
//!     ChildComponent()
//! })
//! ```
//! 子组件可通过 `hooks.use_context::<MyData>()` 获取注入的数据。

use crate::{AnyElement, Component, Context};
use ratatui_kit_macros::Props;

#[derive(Default, Props)]
/// ContextProvider 组件属性。
pub struct ContextProviderProps<'a> {
    /// 子元素列表。
    pub children: Vec<AnyElement<'a>>,
    /// 注入的上下文对象。
    pub value: Option<Context<'a>>,
}

/// ContextProvider 组件实现。
pub struct ContextProvider;

impl Component for ContextProvider {
    type Props<'a> = ContextProviderProps<'a>;
    fn new(_props: &Self::Props<'_>) -> Self {
        Self
    }

    fn update(
        &mut self,
        props: &mut Self::Props<'_>,
        _hooks: crate::Hooks,
        updater: &mut crate::ComponentUpdater,
    ) {
        updater.set_transparent_layout(true);
        updater.update_children(
            props.children.iter_mut(),
            props.value.as_mut().map(|v| v.borrow()),
        );
    }
}
