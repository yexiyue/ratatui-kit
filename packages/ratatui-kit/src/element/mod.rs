mod key;
pub use key::ElementKey;
mod any_element;
pub use any_element::AnyElement;
mod element_ext;
pub use element_ext::ElementExt;
mod element;
pub use element::Element;

pub trait ElementType {
    type Props<'a>
    where
        Self: 'a;
}
