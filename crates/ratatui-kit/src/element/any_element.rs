use super::{Element, ElementKey, element_ext::ElementRepr};
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
            props: AnyProps::owned(value.props, ComponentHelper::<T>::props_type_id()),
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
            props: AnyProps::borrowed(&mut value.props, ComponentHelper::<T>::props_type_id()),
            helper: ComponentHelper::<T>::boxed(),
        }
    }
}

impl<'a, 'b: 'a> From<&'a mut AnyElement<'b>> for AnyElement<'a> {
    fn from(value: &'a mut AnyElement<'b>) -> Self {
        Self {
            key: value.key.clone(),
            props: value.props.borrow(),
            helper: value.helper.copy(),
        }
    }
}

impl<'a> ElementRepr for AnyElement<'a> {
    fn key(&self) -> &ElementKey {
        &self.key
    }

    fn helper(&self) -> Box<dyn ComponentHelperExt> {
        self.helper.copy()
    }

    fn props_mut(&'_ mut self) -> AnyProps<'_> {
        self.props.borrow()
    }
}
