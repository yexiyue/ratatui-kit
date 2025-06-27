use crate::{
    element::ElementType,
    hooks::Hooks,
    layout_style::LayoutStyle,
    props::{AnyProps, Props},
    render::{ComponentDrawer, ComponentUpdater},
};
use std::{any::Any, pin::Pin, task::Context};

mod component_helper;
pub(crate) use component_helper::{ComponentHelper, ComponentHelperExt};

mod instantiated_component;
pub use instantiated_component::{Components, InstantiatedComponent};
use ratatui::layout::{Direction, Layout};

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
        self.render_ref(drawer.area, drawer.buffer_mut());
    }

    // 默认使用flex布局计算子组件的area
    fn calc_children_areas(
        &self,
        children: &Components,
        layout_style: &LayoutStyle,
        drawer: &mut ComponentDrawer<'_, '_>,
    ) -> Vec<ratatui::prelude::Rect> {
        let layout = layout_style
            .get_layout()
            .constraints(children.get_constraints(layout_style.flex_direction));

        let areas = layout.split(drawer.area);

        let mut children_areas: Vec<ratatui::prelude::Rect> = vec![];

        let rev_direction = match layout_style.flex_direction {
            Direction::Horizontal => Direction::Vertical,
            Direction::Vertical => Direction::Horizontal,
        };
        for (area, constraint) in areas.iter().zip(children.get_constraints(rev_direction)) {
            let area = Layout::new(rev_direction, [constraint]).split(*area)[0];
            children_areas.push(area);
        }

        children_areas
    }

    fn poll_change(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> std::task::Poll<()> {
        std::task::Poll::Pending
    }

    fn render_ref(&self, _area: ratatui::layout::Rect, _buf: &mut ratatui::buffer::Buffer) {}
}

pub trait AnyComponent: Any + Send + Sync + Unpin {
    fn update(&mut self, props: AnyProps, hooks: Hooks, updater: &mut ComponentUpdater);

    fn draw(&mut self, drawer: &mut ComponentDrawer);

    fn calc_children_areas(
        &self,
        children: &Components,
        layout_style: &LayoutStyle,
        drawer: &mut ComponentDrawer,
    ) -> Vec<ratatui::prelude::Rect>;

    fn poll_change(self: Pin<&mut Self>, cx: &mut Context) -> std::task::Poll<()>;

    fn render_ref(&self, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer);
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

    fn calc_children_areas(
        &self,
        children: &Components,
        layout_style: &LayoutStyle,
        drawer: &mut ComponentDrawer,
    ) -> Vec<ratatui::prelude::Rect> {
        Component::calc_children_areas(self, children, layout_style, drawer)
    }

    fn poll_change(self: Pin<&mut Self>, cx: &mut Context) -> std::task::Poll<()> {
        Component::poll_change(self, cx)
    }

    fn render_ref(&self, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer) {
        Component::render_ref(self, area, buf);
    }
}
