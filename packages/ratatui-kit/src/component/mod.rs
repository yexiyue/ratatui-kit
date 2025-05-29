use crate::{
    element::ElementType,
    hooks::Hooks,
    props::{AnyProps, Props},
    render::{ComponentDrawer, ComponentUpdater},
};
use std::{any::Any, pin::Pin, task::Context};

mod component_helper;
pub(crate) use component_helper::ComponentHelper;
pub use component_helper::ComponentHelperExt;

pub trait Component: Any + Send + Sync + Unpin {
    type Props<'a>: Props
    where
        Self: 'a;

    fn new(props: &Self::Props<'_>) -> Self;

    fn update(
        &mut self,
        _props: &mut Self::Props<'_>,
        _hooks: Hooks,
        _updater: &mut ComponentUpdater,
    ) {
    }

    fn draw(&mut self, _drawer: &mut ComponentDrawer<'_>) {}

    fn poll_change(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> std::task::Poll<()> {
        std::task::Poll::Pending
    }
}

pub trait AnyComponent: Any + Send + Sync + Unpin {
    fn update(&mut self, props: &mut AnyProps, hooks: Hooks, updater: &mut ComponentUpdater);

    fn draw(&mut self, drawer: &mut ComponentDrawer);

    fn poll_change(self: Pin<&mut Self>, cx: &mut Context) -> std::task::Poll<()>;
}

impl<C> ElementType for C
where
    C: Component,
{
    type Props<'a> = C::Props<'a>;
}

impl<C> AnyComponent for C
where
    C: Any + Component,
{
    fn update(&mut self, props: &mut AnyProps, hooks: Hooks, updater: &mut ComponentUpdater) {
        Component::update(
            self,
            unsafe { props.downcast_mut_unchecked() },
            hooks,
            updater,
        );
    }

    fn draw(&mut self, drawer: &mut ComponentDrawer) {
        Component::draw(self, drawer);
    }

    fn poll_change(self: Pin<&mut Self>, cx: &mut Context) -> std::task::Poll<()> {
        Component::poll_change(self, cx)
    }
}
