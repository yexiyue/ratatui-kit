use crate::{context::ContextStack, terminal::Terminal};

pub struct ComponentUpdater<'a, 'c: 'a> {
    component_context_stack: &'a mut ContextStack<'c>,
    terminal: &'a mut Terminal,
}
