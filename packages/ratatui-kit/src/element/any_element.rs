use super::{Element, ElementKey, element_ext::ElementExt};
use crate::{
    component::{Component, ComponentHelper, ComponentHelperExt},
    props::AnyProps,
};

pub struct AnyElement<'a> {
    key: ElementKey,
    props: AnyProps<'a>,
    helper: Box<dyn ComponentHelperExt>,
}

impl<'a, T> From<Element<'a, T>> for AnyElement<'a>
where
    T: Component,
{
    fn from(value: Element<'a, T>) -> Self {
        Self {
            key: value.key,
            props: AnyProps::owned(value.props),
            helper: ComponentHelper::<T>::boxed(),
        }
    }
}

impl<'a, 'b: 'a, T> From<&'a mut Element<'b, T>> for AnyElement<'a>
where
    T: Component,
{
    fn from(value: &'a mut Element<'b, T>) -> Self {
        Self {
            key: value.key.clone(),
            props: AnyProps::borrowed(&mut value.props),
            helper: ComponentHelper::<T>::boxed(),
        }
    }
}

impl<'a, 'b: 'a> From<&'a mut AnyElement<'b>> for AnyElement<'b> {
    fn from(value: &'a mut AnyElement<'b>) -> Self {
        Self {
            key: value.key.clone(),
            props: value.props.borrow(),
            helper: value.helper.copy(),
        }
    }
}

impl ElementExt for AnyElement<'_> {
    fn key(&self) -> &ElementKey {
        &self.key
    }

    fn helper(&self) -> Box<dyn ComponentHelperExt> {
        self.helper.copy()
    }

    fn props_mut(&mut self) -> AnyProps {
        self.props.borrow()
    }
}
