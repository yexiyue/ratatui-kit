use std::{
    any::Any,
    cell::{Ref, RefMut},
};

use crate::{
    component::{Components, InstantiatedComponent},
    context::{Context, ContextStack},
    element::ElementExt,
    multimap::AppendOnlyMultimap,
    terminal::Terminal,
};

pub struct ComponentUpdater<'a, 'c: 'a> {
    component_context_stack: &'a mut ContextStack<'c>,
    terminal: &'a mut Terminal,
    components: &'a mut Components,
}

impl<'a, 'c: 'a> ComponentUpdater<'a, 'c> {
    pub(crate) fn new(
        component_context_stack: &'a mut ContextStack<'c>,
        terminal: &'a mut Terminal,
        components: &'a mut Components,
    ) -> ComponentUpdater<'a, 'c> {
        ComponentUpdater {
            component_context_stack,
            terminal,
            components,
        }
    }

    pub fn component_context_stack(&self) -> &ContextStack<'c> {
        self.component_context_stack
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
                            InstantiatedComponent::new(child.props_mut(), h)
                        }
                    };

                    component.update(self.terminal, context_stack, child.props_mut());
                    used_components.push_back(child.key().clone(), component);
                }

                self.components.components = used_components.into();
            });
    }
}
