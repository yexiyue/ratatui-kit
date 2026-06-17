//! View 组件：基础布局容器，支持 flex 布局、嵌套、间距、对齐等属性。
//!
//! 常用于包裹和组织多个子组件，是构建 UI 结构的基础。
//!
//! ## 示例
//! ```rust
//! element!(View(flex_direction: Direction::Vertical, gap: 1) {
//!     element!(Child1()),
//!     element!(Child2()),
//! })
//! ```
//! 可通过 `flex_direction`、`gap`、`margin` 等属性灵活控制布局。

use ratatui_kit_macros::{Props, with_layout_style};

use crate::{AnyElement, Component};

#[with_layout_style]
#[derive(Default, Props)]
/// View 组件属性。
pub struct ViewProps<'a> {
    /// 子元素列表。
    pub children: Vec<AnyElement<'a>>,
}

/// View 组件实现。
pub struct View;

impl Component for View {
    type Props<'a> = ViewProps<'a>;

    fn new(_props: &Self::Props<'_>) -> Self {
        Self
    }

    fn update(
        &mut self,
        props: &mut Self::Props<'_>,
        _hooks: crate::Hooks,
        updater: &mut crate::ComponentUpdater,
    ) {
        updater.set_layout_style(props.layout_style());
        updater.update_children(&mut props.children, None);
    }
}
