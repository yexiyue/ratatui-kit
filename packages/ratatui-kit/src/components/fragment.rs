//! Fragment 组件：无额外渲染的透明容器，用于包裹多个子元素，类似 React.Fragment。
//!
//! ## 用法
//! ```rust
//! element!(Fragment {
//!     Child1(),
//!     Child2(),
//! })
//! ```
//! Fragment 不会生成额外的布局节点，常用于返回多个根元素或批量包裹子组件。

use ratatui_kit_macros::Props;

use crate::{AnyElement, Component, ComponentUpdater, Hooks};

#[derive(Default, Props)]
pub struct FragmentProps<'a> {
    /// 子元素列表。
    pub children: Vec<AnyElement<'a>>,
}

#[derive(Default)]
/// Fragment 组件实现。
pub struct Fragment;

impl Component for Fragment {
    type Props<'a> = FragmentProps<'a>;

    fn new(_props: &Self::Props<'_>) -> Self {
        Self
    }

    fn update(
        &mut self,
        props: &mut Self::Props<'_>,
        _hooks: Hooks,
        updater: &mut ComponentUpdater,
    ) {
        updater.set_transparent_layout(true);
        updater.update_children(props.children.iter_mut(), None);
    }
}
