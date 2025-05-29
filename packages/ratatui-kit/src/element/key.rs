use any_key::AnyHash;
use std::{fmt::Debug, hash::Hash, sync::Arc};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ElementKey(Arc<Box<dyn AnyHash + Send + Sync>>);

impl ElementKey {
    pub fn new<T>(key: T) -> Self
    where
        T: Debug + Hash + Send + Sync + Eq + 'static,
    {
        Self(Arc::new(Box::new(key)))
    }
}
