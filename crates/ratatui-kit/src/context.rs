// 上下文模块，提供全局/局部依赖注入能力，支持跨组件数据共享与生命周期管理。
//
// ## 主要类型
// - [`Context`]：通用上下文枚举，支持所有权、不可变/可变引用三种模式。
// - [`ContextStack`]：上下文栈，支持嵌套作用域和动态查找。
// - [`SystemContext`]：系统级上下文，控制全局退出等。

use std::{
    any::{Any, TypeId},
    cell::{Ref, RefCell, RefMut},
};

// 通用上下文类型，支持所有权、不可变引用、可变引用三种模式。
pub enum Context<'a> {
    Ref(&'a dyn Any),
    Mut(&'a mut dyn Any),
    Owned(Box<dyn Any>),
}

impl<'a> Context<'a> {
    // 创建一个拥有所有权的上下文。
    pub fn owned<T: Any>(context: T) -> Self {
        Context::Owned(Box::new(context))
    }

    // 创建一个不可变引用的上下文。
    pub fn from_ref<T: Any>(context: &'a T) -> Self {
        Context::Ref(context)
    }

    // 创建一个可变引用的上下文。
    pub fn from_mut<T: Any>(context: &'a mut T) -> Self {
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

    pub fn borrow(&'_ mut self) -> Context<'_> {
        match self {
            Context::Ref(context) => Context::Ref(*context),
            Context::Mut(context) => Context::Mut(*context),
            Context::Owned(context) => Context::Mut(&mut **context),
        }
    }

    fn type_id(&self) -> TypeId {
        match self {
            Context::Ref(context) => (*context).type_id(),
            Context::Mut(context) => (**context).type_id(),
            Context::Owned(context) => (**context).type_id(),
        }
    }
}

struct ContextEntry<'a> {
    type_id: TypeId,
    context: RefCell<Context<'a>>,
}

impl<'a> ContextEntry<'a> {
    fn new(context: Context<'a>) -> Self {
        Self {
            type_id: context.type_id(),
            context: RefCell::new(context),
        }
    }
}

// `ContextStack` 查找结果——区分三态,使断言型 `use_context` 给出精确诊断,
// 而 `try_use_context` 能安全降级为 `None`(不 panic)。
pub(crate) enum ContextLookup<R> {
    // 找到且成功借用。
    Found(R),
    // 类型匹配但当前已被借用(持守卫重入,属编程错误)。
    AlreadyBorrowed,
    // 栈中无该类型 context。
    NotFound,
}

pub struct ContextStack<'a> {
    stack: Vec<ContextEntry<'a>>,
}

impl<'a> ContextStack<'a> {
    pub(crate) fn root(root_context: &'a mut dyn Any) -> Self {
        ContextStack {
            stack: vec![ContextEntry::new(Context::Mut(root_context))],
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
            shorter_lived_self.stack.push(ContextEntry::new(context));
            f(shorter_lived_self);
            shorter_lived_self.stack.pop();
        } else {
            f(self);
        };
    }

    pub(crate) fn get_context<T: Any>(&'_ self) -> ContextLookup<Ref<'_, T>> {
        let expected_type_id = TypeId::of::<T>();
        for entry in self.stack.iter().rev() {
            if entry.type_id != expected_type_id {
                continue;
            }

            let Ok(context) = entry.context.try_borrow() else {
                return ContextLookup::AlreadyBorrowed;
            };

            if let Ok(res) = Ref::filter_map(context, |context| context.downcast_ref::<T>()) {
                return ContextLookup::Found(res);
            }
        }
        ContextLookup::NotFound
    }

    pub(crate) fn get_context_mut<T: Any>(&'_ self) -> ContextLookup<RefMut<'_, T>> {
        let expected_type_id = TypeId::of::<T>();
        for entry in self.stack.iter().rev() {
            if entry.type_id != expected_type_id {
                continue;
            }

            let Ok(context) = entry.context.try_borrow_mut() else {
                return ContextLookup::AlreadyBorrowed;
            };

            if let Ok(res) = RefMut::filter_map(context, |context| context.downcast_mut::<T>()) {
                return ContextLookup::Found(res);
            }
        }
        ContextLookup::NotFound
    }
}

pub struct SystemContext {
    should_exit: bool,
    /// 收到 Ctrl+C 键盘事件时是否自动退出 fullscreen event loop。
    /// 默认 true（向后兼容）。若设为 false，Ctrl+C 仍会经 input.dispatch 分发给
    /// Global handler，由应用层自行决定行为（如双击退出、取消 agent 等）。
    pub auto_quit_on_ctrl_c: bool,
    // 中央输入事件运行时。组件经 `get_context_mut::<SystemContext>().input` 登记层/handler,
    // 渲染循环经 `system_context.input.dispatch(event)` 分发。运行时单线程,无需 Send + Sync。
    pub(crate) input: crate::input::InputRuntime,
}

impl SystemContext {
    pub(crate) fn new() -> Self {
        Self {
            should_exit: false,
            auto_quit_on_ctrl_c: true, // 默认 true，保持向后兼容
            input: crate::input::InputRuntime::default(),
        }
    }

    pub(crate) fn should_exit(&self) -> bool {
        self.should_exit
    }

    pub fn exit(&mut self) {
        self.should_exit = true;
    }
}
