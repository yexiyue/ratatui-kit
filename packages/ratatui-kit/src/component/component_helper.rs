use std::any::{Any, TypeId};

use crate::{hooks::Hooks, props::AnyProps, render::ComponentUpdater};

use super::{AnyComponent, Component};

pub trait ComponentHelperExt: Any + Send + Sync {
    fn new_component(&self, props: AnyProps) -> Box<dyn AnyComponent>;

    fn update_component(
        &self,
        component: &mut Box<dyn AnyComponent>,
        props: &mut AnyProps,
        hooks: Hooks,
        updater: &mut ComponentUpdater,
    );

    fn component_type_id(&self) -> TypeId;

    fn copy(&self) -> Box<dyn ComponentHelperExt>;
}

pub(crate) struct ComponentHelper<T>
where
    T: Component,
{
    _marker: std::marker::PhantomData<T>,
}

impl<T> ComponentHelper<T>
where
    T: Component,
{
    pub fn boxed() -> Box<dyn ComponentHelperExt> {
        Box::new(Self {
            _marker: std::marker::PhantomData,
        })
    }
}

impl<T> ComponentHelperExt for ComponentHelper<T>
where
    T: Component,
{
    fn new_component(&self, props: AnyProps) -> Box<dyn AnyComponent> {
        Box::new(T::new(unsafe { props.downcast_ref_unchecked() }))
    }
    fn copy(&self) -> Box<dyn ComponentHelperExt> {
        Self::boxed()
    }

    fn component_type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn update_component(
        &self,
        component: &mut Box<dyn AnyComponent>,
        props: &mut AnyProps,
        hooks: Hooks,
        updater: &mut ComponentUpdater,
    ) {
        component.update(props, hooks, updater);
    }
}
