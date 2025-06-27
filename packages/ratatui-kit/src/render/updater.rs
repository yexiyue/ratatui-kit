use std::{
    any::Any,
    cell::{Ref, RefMut},
};

use crate::{
    ElementKey,
    component::{Components, InstantiatedComponent},
    context::{Context, ContextStack},
    element::ElementExt,
    layout_style::LayoutStyle,
    multimap::AppendOnlyMultimap,
    terminal::Terminal,
};

pub struct ComponentUpdater<'a, 'c: 'a> {
    key: ElementKey,
    component_context_stack: &'a mut ContextStack<'c>,
    terminal: &'a mut Terminal,
    components: &'a mut Components,
    transparent_layout: bool,
    layout_style: &'a mut LayoutStyle,
}

impl<'a, 'c: 'a> ComponentUpdater<'a, 'c> {
    pub(crate) fn new(
        key: ElementKey,
        component_context_stack: &'a mut ContextStack<'c>,
        terminal: &'a mut Terminal,
        components: &'a mut Components,
        layout_style: &'a mut LayoutStyle,
    ) -> ComponentUpdater<'a, 'c> {
        ComponentUpdater {
            key,
            component_context_stack,
            terminal,
            components,
            transparent_layout: false,
            layout_style,
        }
    }

    pub fn component_context_stack(&self) -> &ContextStack<'c> {
        self.component_context_stack
    }

    pub fn key(&self) -> &ElementKey {
        &self.key
    }

    pub fn get_context<T: Any>(&self) -> Option<Ref<T>> {
        self.component_context_stack.get_context()
    }

    pub fn get_context_mut<T: Any>(&self) -> Option<RefMut<T>> {
        self.component_context_stack.get_context_mut()
    }

    pub fn terminal(&mut self) -> &mut Terminal {
        self.terminal
    }

    pub fn set_transparent_layout(&mut self, transparent: bool) {
        self.transparent_layout = transparent;
    }

    pub(crate) fn has_transparent_layout(&self) -> bool {
        self.transparent_layout
    }

    pub fn set_layout_style(&mut self, layout_style: LayoutStyle) {
        *self.layout_style = layout_style;
    }

    pub fn update_children<I, T>(&mut self, elements: I, context: Option<Context>)
    where
        I: IntoIterator<Item = T>,
        T: ElementExt,
    {
        self.component_context_stack
            .with_context(context, |context_stack| {
                let mut used_components = AppendOnlyMultimap::default();

                for mut child in elements {
                    let mut component = match self.components.pop_front(child.key()) {
                        Some(component)
                            if component.component().type_id()
                                == child.helper().component_type_id() =>
                        {
                            component
                        }
                        _ => {
                            let h = child.helper();
                            InstantiatedComponent::new(child.key().clone(), child.props_mut(), h)
                        }
                    };

                    component.update(self.terminal, context_stack, child.props_mut());
                    used_components.push_back(child.key().clone(), component);
                }

                self.components.components = used_components.into();
            });
    }
}
