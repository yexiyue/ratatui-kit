pub mod key;
pub use key::ElementKey;
pub mod any_element;
pub use any_element::AnyElement;

use crate::component::Component;

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
