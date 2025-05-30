use crate::component::Component;

mod key;
pub use key::ElementKey;
mod any_element;
pub use any_element::AnyElement;
mod element_ext;
pub use element_ext::ElementExt;

pub trait ElementType {
    type Props<'a>
    where
        Self: 'a;
}

#[derive(Clone)]
pub struct Element<'a, T: ElementType + 'a> {
    key: ElementKey,
    props: T::Props<'a>,
}

impl<'a, T> Element<'a, T>
where
    T: Component + 'a,
{
    pub fn into_any(self) -> AnyElement<'a> {
        self.into()
    }
}
