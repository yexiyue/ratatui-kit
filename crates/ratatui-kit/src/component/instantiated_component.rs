use super::{AnyComponent, ComponentHelperExt};
use crate::{
    context::ContextStack,
    element::ElementKey,
    hooks::{AnyHook, Hook, Hooks},
    multimap::RemoveOnlyMultimap,
    props::AnyProps,
    render::{ComponentDrawer, ComponentUpdater, layout_style::LayoutStyle},
    terminal::UpdaterTerminal,
};
use ratatui::layout::{Constraint, Direction};
use std::{
    future::poll_fn,
    ops::{Deref, DerefMut},
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

    fn poll_change(&mut self, cx: &mut Context) -> Poll<()> {
        let mut is_ready = false;
        for component in self.components.iter_mut() {
            if component.poll_change(cx).is_ready() {
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

#[doc(hidden)]
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
        terminal: &mut dyn UpdaterTerminal,
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

        if self.has_transparent_layout {
            if let Some(child) = self.children.iter().next() {
                self.layout_style = child.layout_style.clone();
            } else {
                self.layout_style = LayoutStyle::default();
            }
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
        debug_assert_eq!(
            children_areas.len(),
            self.children.components.iter().count(),
            "calc_children_areas must return one area per child"
        );

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

    pub(crate) fn poll_change(&mut self, cx: &mut Context) -> Poll<()> {
        // 三路必须全部 poll,即使前一路已 Ready 也不能短路;否则 Pending 的路
        // 无法在本轮注册 waker,后续变更会丢唤醒。
        let component_status = self.component.poll_change(cx);
        let children_status = self.children.poll_change(cx);
        let hooks_status = self.hooks.poll_change(cx);
        if component_status.is_ready() || children_status.is_ready() || hooks_status.is_ready() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }

    pub async fn wait(&mut self) {
        poll_fn(|cx| self.poll_change(cx)).await;
    }
}

impl Drop for InstantiatedComponent {
    fn drop(&mut self) {
        self.hooks.on_drop();
    }
}
