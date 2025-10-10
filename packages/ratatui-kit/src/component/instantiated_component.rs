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
pub struct Components {
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

pub struct InstantiatedComponent {
    key: ElementKey,
    hooks: Vec<Box<dyn AnyHook>>,
    component: Box<dyn AnyComponent>,
    helper: Box<dyn ComponentHelperExt>,
    children: Components,
    first_update: bool,
    layout_style: LayoutStyle,
    has_transparent_layout: bool,
}

impl InstantiatedComponent {
    pub fn new(key: ElementKey, mut props: AnyProps, helper: Box<dyn ComponentHelperExt>) -> Self {
        let component = helper.new_component(props.borrow());
        Self {
            key,
            hooks: Default::default(),
            layout_style: LayoutStyle::default(),
            component,
            children: Components::default(),
            helper,
            first_update: true,
            has_transparent_layout: false,
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
        let mut updater = ComponentUpdater::new(
            self.key.clone(),
            context_stack,
            terminal,
            &mut self.children,
            &mut self.layout_style,
        );
        self.hooks.pre_component_update(&mut updater);
        self.helper.update_component(
            &mut self.component,
            props.borrow(),
            Hooks::new(&mut self.hooks, self.first_update),
            &mut updater,
        );
        self.hooks.post_component_update(&mut updater);
        self.first_update = false;
        self.has_transparent_layout = updater.has_transparent_layout();

        // 如果是透明布局，则继承第一个子组件的布局属性
        if let Some(child) = self.children.iter().next()
            && self.has_transparent_layout
        {
            self.layout_style = child.layout_style.clone();
        }
    }

    pub fn draw(&mut self, drawer: &mut ComponentDrawer) {
        let layout_style = &self.layout_style;

        let area = if self.has_transparent_layout {
            drawer.area
        } else {
            layout_style.inner_area(drawer.area)
        };

        drawer.area = area;

        // 先渲染在计算子组件的areas
        self.hooks.pre_component_draw(drawer);

        // drawer.ares可能在组件绘制时改变
        self.component.draw(drawer);

        // 计算子组件的区域
        let children_areas =
            self.component
                .calc_children_areas(&self.children, layout_style, drawer);

        for (child, area) in self
            .children
            .components
            .iter_mut()
            .zip(children_areas.iter())
        {
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

impl Drop for InstantiatedComponent {
    fn drop(&mut self) {
        self.hooks.on_drop();
    }
}
