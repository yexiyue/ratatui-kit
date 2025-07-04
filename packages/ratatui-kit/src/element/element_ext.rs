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

pub trait ElementExt: private::Sealed + Sized {
    fn key(&self) -> &ElementKey;
    fn props_mut(&mut self) -> AnyProps;
    fn helper(&self) -> Box<dyn ComponentHelperExt>;
    fn render_loop(&mut self, options: TerminalOptions) -> impl Future<Output = io::Result<()>>;
    fn fullscreen(&mut self) -> impl Future<Output = io::Result<()>>;
}
