use crate::{component::ComponentHelperExt, props::AnyProps};

use super::ElementKey;

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

pub trait ElementExt: private::Sealed {
    fn key(&self) -> &ElementKey;
    fn props_mut(&mut self) -> AnyProps;
    fn helper(&self) -> Box<dyn ComponentHelperExt>;
}
