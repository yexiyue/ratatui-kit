mod key;
pub use key::ElementKey;
mod any_element;
pub use any_element::AnyElement;
mod element_ext;
pub use element_ext::ElementExt;
#[allow(clippy::module_inception)]
mod element;
pub use element::Element;
mod extend_with_elements;
pub use extend_with_elements::{ExtendWithElements, extend_with_elements};

pub trait ElementType {
    type Props<'a>
    where
        Self: 'a;
}
