use std::{
    any::Any,
    cell::{Ref, RefMut},
};

use crate::{component::Components, context::ContextStack, terminal::Terminal};

pub struct ComponentUpdater<'a, 'c: 'a> {
    component_context_stack: &'a mut ContextStack<'c>,
    terminal: &'a mut Terminal,
    components: &'a mut Components,
}

impl<'a, 'c: 'a> ComponentUpdater<'a, 'c> {
    pub fn new(
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
}
