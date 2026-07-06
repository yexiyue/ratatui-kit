use crate::{AnyProps, Component, ComponentHelper, ComponentHelperExt};
mod key;
pub use key::ElementKey;
mod any_element;
pub use any_element::AnyElement;
mod element_ext;
pub use element_ext::{ElementExt, ElementRepr};
mod extend_with_elements;
pub use extend_with_elements::{ExtendWithElements, extend_with_elements};

#[doc(hidden)]
pub trait ElementType {
    type Props<'a>
    where
        Self: 'a;
}

#[derive(Clone)]
pub struct Element<'a, T: ElementType + 'a> {
    pub key: ElementKey,
    pub props: T::Props<'a>,
}

impl<'a, T> Element<'a, T>
where
    T: Component + 'a,
{
    pub fn into_any(self) -> AnyElement<'a> {
        self.into()
    }
}

impl<'a, T> ElementRepr for Element<'a, T>
where
    T: Component,
{
    fn key(&self) -> &ElementKey {
        &self.key
    }

    fn helper(&self) -> Box<dyn ComponentHelperExt> {
        ComponentHelper::<T>::boxed()
    }

    fn props_mut(&'_ mut self) -> AnyProps<'_> {
        AnyProps::borrowed(&mut self.props, ComponentHelper::<T>::props_type_id())
    }
}
