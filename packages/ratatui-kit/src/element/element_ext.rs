use ratatui::TerminalOptions;

use super::ElementKey;
use crate::{component::ComponentHelperExt, props::AnyProps};
use std::io;

mod private {
    use crate::{
        component::Component,
        element::{AnyElement, Element},
    };

    pub trait Sealed {}

    impl<'a> Sealed for AnyElement<'a> {}
    impl<'a> Sealed for &mut AnyElement<'a> {}

    impl<'a, T> Sealed for Element<'a, T> where T: Component {}
    impl<'a, T> Sealed for &mut Element<'a, T> where T: Component {}
}

/// ElementExt trait 为所有 UI 元素提供统一的扩展方法。
///
/// 支持获取/修改 key、props，辅助渲染，启动主循环、全屏渲染等。
/// 适用于组件树的统一操作和终端 UI 应用的入口。
///
/// # 常用用法
/// ```rust
/// element!(MyComponent).fullscreen().await?;
/// ```
pub trait ElementExt: private::Sealed + Sized {
    /// 获取元素的唯一 key，适合 diff、重用等场景。
    fn key(&self) -> &ElementKey;
    /// 获取并可变修改元素的属性（props）。
    fn props_mut(&mut self) -> AnyProps;
    /// 获取组件辅助操作对象，支持动态调度和扩展。
    fn helper(&self) -> Box<dyn ComponentHelperExt>;
    /// 启动渲染主循环，传入终端选项，适合自定义Viewport场景。
    fn render_loop(&mut self, options: TerminalOptions) -> impl Future<Output = io::Result<()>>;
    /// 以全屏模式运行当前元素，适合大多数终端 UI 应用入口。
    fn fullscreen(&mut self) -> impl Future<Output = io::Result<()>>;
}
