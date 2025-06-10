use core::ops::{Deref, DerefMut};

pub struct Handler<'a, T>(bool, Box<dyn FnMut(T) + Send + Sync + 'a>);

impl<T> Handler<'_, T> {
    pub fn is_default(&self) -> bool {
        !self.0
    }

    pub fn take(&mut self) -> Self {
        core::mem::take(self)
    }
}

impl<'a, T> Default for Handler<'a, T> {
    fn default() -> Self {
        Self(false, Box::new(|_| {}))
    }
}

impl<'a, F, T> From<F> for Handler<'a, T>
where
    F: FnMut(T) + Send + Sync + 'a,
{
    fn from(f: F) -> Self {
        Self(false, Box::new(f))
    }
}

impl<'a, T> Deref for Handler<'a, T> {
    type Target = Box<dyn FnMut(T) + Send + Sync + 'a>;

    fn deref(&self) -> &Self::Target {
        &self.1
    }
}

impl<'a, T> DerefMut for Handler<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.1
    }
}
