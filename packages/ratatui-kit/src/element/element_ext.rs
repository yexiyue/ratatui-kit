use super::ElementKey;
use crate::{component::ComponentHelperExt, props::AnyProps};
use std::io;

mod private {
    use crate::{
        component::Component,
        element::{AnyElement, Element},
    };

    pub trait Sealed {}

    impl Sealed for AnyElement<'_> {}
    impl Sealed for &mut AnyElement<'_> {}

    impl<T> Sealed for Element<'_, T> where T: Component {}
    impl<T> Sealed for &mut Element<'_, T> where T: Component {}
}

pub trait ElementExt: private::Sealed + Sized {
    fn key(&self) -> &ElementKey;
    fn props_mut(&mut self) -> AnyProps;
    fn helper(&self) -> Box<dyn ComponentHelperExt>;
    fn render(&mut self) -> io::Result<()>;
    fn render_loop(&mut self) -> impl Future<Output = io::Result<()>>;
    fn fullscreen(&mut self) -> impl Future<Output = io::Result<()>>;
}
