use super::{AnyComponent, ComponentHelperExt};
use crate::{
    context::ContextStack,
    element::ElementKey,
    hooks::{AnyHook, Hook, Hooks},
    multimap::RemoveOnlyMultimap,
    props::AnyProps,
    render::{ComponentDrawer, ComponentUpdater, layout_style::LayoutStyle},
    terminal::Terminal,
};
use ratatui::layout::{Constraint, Direction};
use std::{
    future::poll_fn,
    ops::{Deref, DerefMut},
    pin::Pin,
    task::{Context, Poll},
};

#[derive(Default)]
pub(crate) struct Components {
    pub components: RemoveOnlyMultimap<ElementKey, InstantiatedComponent>,
}

impl Deref for Components {
    type Target = RemoveOnlyMultimap<ElementKey, InstantiatedComponent>;

    fn deref(&self) -> &Self::Target {
        &self.components
    }
}

impl DerefMut for Components {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.components
    }
}

impl Components {
    pub fn get_constraints(&self, direction: Direction) -> Vec<Constraint> {
        self.components
            .iter()
            .map(|c| match direction {
                Direction::Horizontal => c.layout_style.get_width(),
                Direction::Vertical => c.layout_style.get_height(),
            })
            .collect()
    }

    fn poll_change(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<()> {
        let mut is_ready = false;
        for component in self.components.iter_mut() {
            if Pin::new(component).poll_change(cx).is_ready() {
                is_ready = true;
            }
        }

        if is_ready {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

pub(crate) struct InstantiatedComponent {
    hooks: Vec<Box<dyn AnyHook>>,
    component: Box<dyn AnyComponent>,
    helper: Box<dyn ComponentHelperExt>,
    children: Components,
    first_update: bool,
    layout_style: LayoutStyle,
}

impl InstantiatedComponent {
    pub fn new(mut props: AnyProps, helper: Box<dyn ComponentHelperExt>) -> Self {
        let component = helper.new_component(props.borrow());
        Self {
            hooks: Default::default(),
            layout_style: component.get_layout_style(props),
            component,
            children: Components::default(),
            helper,
            first_update: true,
        }
    }

    pub fn component(&self) -> &dyn AnyComponent {
        &*self.component
    }

    pub fn update(
        &mut self,
        terminal: &mut Terminal,
        context_stack: &mut ContextStack,
        mut props: AnyProps,
    ) {
        let mut updater = ComponentUpdater::new(context_stack, terminal, &mut self.children);
        self.hooks.pre_component_update(&mut updater);
        self.helper.update_component(
            &mut self.component,
            props.borrow(),
            Hooks::new(&mut self.hooks, self.first_update),
            &mut updater,
        );
        self.layout_style = self.component.get_layout_style(props);
        self.hooks.post_component_update(&mut updater);
        self.first_update = false;
    }

    pub fn draw(&mut self, drawer: &mut ComponentDrawer) {
        let area = self.layout_style.inner_area(drawer.area);

        let layout = self.layout_style.get_layout().constraints(
            self.children
                .get_constraints(self.layout_style.flex_direction),
        );
        let areas = layout.split(area);
        drawer.area = area;

        self.hooks.pre_component_draw(drawer);
        self.component.draw(drawer);

        for (child, area) in self.children.components.iter_mut().zip(areas.iter()) {
            drawer.area = *area;
            child.draw(drawer);
        }
        self.hooks.post_component_draw(drawer);
    }

    pub(crate) fn poll_change(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<()> {
        let component_status = Pin::new(&mut *self.component).poll_change(cx);
        let children_status = Pin::new(&mut self.children).poll_change(cx);
        let hooks_status = Pin::new(&mut self.hooks).poll_change(cx);
        if component_status.is_ready() || children_status.is_ready() || hooks_status.is_ready() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }

    pub async fn wait(&mut self) {
        let mut self_mut = Pin::new(self);
        poll_fn(|cx| self_mut.as_mut().poll_change(cx)).await;
    }
}
