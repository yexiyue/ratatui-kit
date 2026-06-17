use ratatui::TerminalOptions;

use super::ElementKey;
use crate::{
    component::ComponentHelperExt,
    props::AnyProps,
    render::tree::render_loop,
    terminal::{CrossTerminal, Terminal},
};
use std::{future::Future, io};

mod private {
    use crate::{
        component::Component,
        element::{AnyElement, Element},
    };

    pub trait Sealed {}

    impl<'a> Sealed for AnyElement<'a> {}
    impl<'a, T> Sealed for Element<'a, T> where T: Component {}
    impl<T: Sealed + ?Sized> Sealed for &mut T {}
}

#[doc(hidden)]
pub trait ElementRepr: private::Sealed + Sized {
    // 获取元素的唯一 key，适合 diff、重用等场景。
    fn key(&self) -> &ElementKey;
    // 获取并可变修改元素的属性（props）。
    fn props_mut(&'_ mut self) -> AnyProps<'_>;
    // 获取组件辅助操作对象，支持动态调度和扩展。
    fn helper(&self) -> Box<dyn ComponentHelperExt>;
}

impl<T> ElementRepr for &mut T
where
    T: ElementRepr,
{
    fn key(&self) -> &ElementKey {
        (**self).key()
    }

    fn props_mut(&'_ mut self) -> AnyProps<'_> {
        (**self).props_mut()
    }

    fn helper(&self) -> Box<dyn ComponentHelperExt> {
        (**self).helper()
    }
}

// ElementExt trait 为所有 UI 元素提供应用入口方法。
//
// # 常用用法
// ```rust
// element!(MyComponent).fullscreen().await?;
// ```
pub trait ElementExt: ElementRepr {
    // 启动渲染主循环，传入终端选项，适合自定义Viewport场景。
    fn render_loop(&mut self, options: TerminalOptions) -> impl Future<Output = io::Result<()>> {
        async move {
            let terminal = Terminal::new(CrossTerminal::with_options(options)?)?;
            render_loop(self, terminal).await?;
            Ok(())
        }
    }

    // 以全屏模式运行当前元素，适合大多数终端 UI 应用入口。
    fn fullscreen(&mut self) -> impl Future<Output = io::Result<()>> {
        async move {
            let terminal = Terminal::new(CrossTerminal::new()?)?;
            render_loop(self, terminal).await?;
            Ok(())
        }
    }
}

impl<T> ElementExt for T where T: ElementRepr {}
