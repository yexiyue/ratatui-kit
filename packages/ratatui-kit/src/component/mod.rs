use crate::{
    element::ElementType,
    hooks::Hooks,
    props::{AnyProps, Props},
    render::{ComponentDrawer, ComponentUpdater, layout_style::LayoutStyle},
};
use std::{any::Any, pin::Pin, task::Context};

mod component_helper;
pub(crate) use component_helper::{ComponentHelper, ComponentHelperExt};

mod instantiated_component;
pub(crate) use instantiated_component::{Components, InstantiatedComponent};

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

    fn draw(&mut self, drawer: &mut ComponentDrawer<'_, '_>) {
        self.render_ref(drawer.area, drawer.frame.buffer_mut());
    }

    fn poll_change(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> std::task::Poll<()> {
        std::task::Poll::Pending
    }

    fn render_ref(&self, _area: ratatui::layout::Rect, _buf: &mut ratatui::buffer::Buffer) {}

    fn get_layout_style(&self, _props: &Self::Props<'_>) -> LayoutStyle {
        LayoutStyle::default()
    }
}

pub trait AnyComponent: Any + Send + Sync + Unpin {
    fn update(&mut self, props: AnyProps, hooks: Hooks, updater: &mut ComponentUpdater);

    fn draw(&mut self, drawer: &mut ComponentDrawer);

    fn poll_change(self: Pin<&mut Self>, cx: &mut Context) -> std::task::Poll<()>;

    fn render_ref(&self, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer);

    fn get_layout_style(&self, props: AnyProps) -> LayoutStyle;
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
    fn update(&mut self, mut props: AnyProps, hooks: Hooks, updater: &mut ComponentUpdater) {
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

    fn render_ref(&self, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer) {
        Component::render_ref(self, area, buf);
    }

    fn get_layout_style(&self, props: AnyProps) -> LayoutStyle {
        Component::get_layout_style(self, unsafe { props.downcast_ref_unchecked() })
    }
}
