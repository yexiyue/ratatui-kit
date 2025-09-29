use super::{AnyElement, Element, ElementType};

pub trait ExtendWithElements<T> {
    fn extend_with_elements<E: Extend<T>>(self, dest: &mut E);
}

impl<'a, T, U> ExtendWithElements<T> for Element<'a, U>
where
    U: ElementType + 'a,
    T: From<Element<'a, U>>,
{
    fn extend_with_elements<E: Extend<T>>(self, dest: &mut E) {
        dest.extend([self.into()]);
    }
}

impl<'a> ExtendWithElements<AnyElement<'a>> for AnyElement<'a> {
    fn extend_with_elements<E: Extend<AnyElement<'a>>>(self, dest: &mut E) {
        dest.extend([self]);
    }
}

impl<'a> ExtendWithElements<AnyElement<'a>> for &mut AnyElement<'a> {
    fn extend_with_elements<E: Extend<AnyElement<'a>>>(self, dest: &mut E) {
        dest.extend([self.into()]);
    }
}

impl<T, U, I> ExtendWithElements<T> for I
where
    T: From<U>,
    I: IntoIterator<Item = U>,
{
    fn extend_with_elements<E: Extend<T>>(self, dest: &mut E) {
        dest.extend(self.into_iter().map(|x| x.into()));
    }
}

pub fn extend_with_elements<T, U, E>(dest: &mut T, elements: U)
where
    U: ExtendWithElements<E>,
    T: Extend<E>,
{
    elements.extend_with_elements(dest);
}
