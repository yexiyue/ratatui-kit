use std::{
    any::Any,
    cell::{Ref, RefCell, RefMut},
};

pub enum Context<'a> {
    Ref(&'a (dyn Any + Send + Sync)),
    Mut(&'a mut (dyn Any + Send + Sync)),
    Owned(Box<dyn Any + Send + Sync>),
}

impl<'a> Context<'a> {
    pub fn owned<T: Any + Send + Sync>(context: T) -> Self {
        Context::Owned(Box::new(context))
    }

    pub fn form_ref<T: Any + Send + Sync>(context: &'a T) -> Self {
        Context::Ref(context)
    }

    pub fn form_mut<T: Any + Send + Sync>(context: &'a mut T) -> Self {
        Context::Mut(context)
    }

    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        match self {
            Context::Ref(context) => context.downcast_ref(),
            Context::Mut(context) => context.downcast_ref(),
            Context::Owned(context) => context.downcast_ref(),
        }
    }

    pub fn downcast_mut<T: Any>(&mut self) -> Option<&mut T> {
        match self {
            Context::Ref(_) => None,
            Context::Mut(context) => context.downcast_mut(),
            Context::Owned(context) => context.downcast_mut(),
        }
    }

    pub fn borrow(&mut self) -> Context {
        match self {
            Context::Ref(context) => Context::Ref(*context),
            Context::Mut(context) => Context::Mut(*context),
            Context::Owned(context) => Context::Mut(&mut **context),
        }
    }
}

pub struct ContextStack<'a> {
    stack: Vec<RefCell<Context<'a>>>,
}

impl<'a> ContextStack<'a> {
    pub(crate) fn root(root_context: &'a mut (dyn Any + Send + Sync)) -> Self {
        ContextStack {
            stack: vec![RefCell::new(Context::Mut(root_context))],
        }
    }
    // 在上下文栈中临时插入一个新的上下文，并在闭包 f 执行期间可用。
    pub(crate) fn with_context<'b, F>(&'b mut self, context: Option<Context<'b>>, f: F)
    where
        F: FnOnce(&mut ContextStack),
    {
        if let Some(context) = context {
            // SAFETY: 可变引用在生命周期上是不变的，为了插入更短生命周期的上下文，需要对 'a 进行转变。
            // 只有在不允许对栈进行其他更改，并且在调用后立即恢复栈的情况下才是安全的。
            let shorter_lived_self =
                unsafe { std::mem::transmute::<&mut Self, &mut ContextStack<'b>>(self) };
            shorter_lived_self.stack.push(RefCell::new(context));
            f(shorter_lived_self);
            shorter_lived_self.stack.pop();
        } else {
            f(self);
        };
    }

    pub fn get_context<T: Any>(&self) -> Option<Ref<T>> {
        for context in self.stack.iter().rev() {
            if let Ok(context) = context.try_borrow() {
                if let Ok(res) = Ref::filter_map(context, |context| context.downcast_ref::<T>()) {
                    return Some(res);
                }
            }
        }
        None
    }

    pub fn get_context_mut<T: Any>(&self) -> Option<RefMut<T>> {
        for context in self.stack.iter().rev() {
            if let Ok(context) = context.try_borrow_mut() {
                if let Ok(res) = RefMut::filter_map(context, |context| context.downcast_mut::<T>())
                {
                    return Some(res);
                }
            }
        }
        None
    }
}

pub struct SystemContext {
    should_exit: bool,
}

impl SystemContext {
    pub(crate) fn new() -> Self {
        Self { should_exit: false }
    }

    pub(crate) fn should_exit(&self) -> bool {
        self.should_exit
    }

    pub fn exit(&mut self) {
        self.should_exit = true;
    }
}
