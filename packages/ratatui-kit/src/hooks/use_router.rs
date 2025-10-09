use std::{
    cell::{Ref, RefMut},
    collections::HashMap,
    sync::Arc,
};

use crate::{
    Handler, State, UseContext,
    prelude::{Route, RouteContext, RouteState, history::RouterHistory},
};

mod private {
    pub trait Sealed {}
    impl Sealed for crate::Hooks<'_, '_> {}
}

pub trait UseRouter<'a>: private::Sealed {
    /// 获取路由跳转器，可用于页面跳转、返回等。
    fn use_navigate(&mut self) -> Navigate;
    /// 获取当前路由状态，适合页面间状态传递。
    fn try_use_route_state<T: Send + Sync + 'static>(&self) -> Option<Arc<T>>;

    fn use_route_state<T: Send + Sync + 'static>(&self) -> Arc<T>;
    /// 获取当前路由信息。
    fn use_route(&self) -> Ref<'a, Route>;
    /// 获取当前路由的可变引用。
    fn use_route_mut(&mut self) -> RefMut<'a, Route>;
    /// 获取当前路由参数。
    fn use_params(&self) -> Ref<'a, HashMap<String, String>>;
}

impl<'a> UseRouter<'a> for crate::Hooks<'a, '_> {
    fn use_navigate(&mut self) -> Navigate {
        let history = self.use_context::<State<RouterHistory>>();
        Navigate::new(*history)
    }

    fn try_use_route_state<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        let route_context = self.use_context::<RouteContext>();

        route_context
            .state
            .as_ref()
            .cloned()
            .and_then(|p| p.downcast::<T>())
    }

    fn use_route_state<T: Send + Sync + 'static>(&self) -> Arc<T> {
        self.try_use_route_state::<T>()
            .expect("route state not found or type mismatch")
    }

    fn use_route(&self) -> Ref<'a, Route> {
        self.use_context::<Route>()
    }

    fn use_route_mut(&mut self) -> RefMut<'a, Route> {
        self.use_context_mut::<Route>()
    }

    fn use_params(&self) -> Ref<'a, HashMap<String, String>> {
        let ctx = self.use_context::<RouteContext>();
        Ref::map(ctx, |c| &c.params)
    }
}

/// 路由跳转器，提供 push、replace、go、back、forward 等方法进行页面导航。
///
/// 类似于 React Router 的 `useNavigate`，可用于主动跳转、带参数跳转、历史记录操作等，适合终端 UI 场景下的路由控制。
#[derive(Clone, Copy)]
pub struct Navigate {
    history: State<RouterHistory>,
}

impl Navigate {
    /// 创建新的 Navigate 实例（内部使用）。
    pub(crate) fn new(history: State<RouterHistory>) -> Self {
        Navigate { history }
    }

    /// 跳转到指定路径，类似于 React Router 的 navigate(path)。
    /// 会将新页面加入历史栈，可用于页面跳转。
    pub fn push(&mut self, path: &str) {
        let mut history = self.history.write();
        let mut ctx = history.current_context();
        ctx.path = path.to_string();
        history.push(ctx);
    }

    /// 跳转到指定路径并携带状态，适合页面间传递数据。
    /// 类似于 React Router 的 navigate(path, { state })。
    pub fn push_with_state<T>(&mut self, path: &str, state: T)
    where
        T: Send + Sync + 'static,
    {
        let mut history = self.history.write();
        let mut ctx = history.current_context();
        ctx.path = path.to_string();
        ctx.state = Some(RouteState::new(state));
        history.push(ctx);
    }

    /// 替换当前页面为指定路径，不会新增历史记录。
    /// 类似于 React Router 的 replace(path)。
    pub fn replace(&mut self, path: &str) {
        let mut history = self.history.write();
        let mut ctx = history.current_context();
        ctx.path = path.to_string();
        history.replace(ctx);
    }

    /// 替换当前页面为指定路径并携带状态。
    /// 类似于 React Router 的 replace(path, { state })。
    pub fn replace_with_state<T>(&mut self, path: &str, state: T)
    where
        T: Send + Sync + 'static,
    {
        let mut history = self.history.write();
        let mut ctx = history.current_context();
        ctx.path = path.to_string();
        ctx.state = Some(RouteState::new(state));
        history.replace(ctx);
    }

    /// 按历史栈偏移跳转，delta > 0 前进，< 0 后退。
    /// 类似于浏览器 history.go(delta)。
    pub fn go(&mut self, delta: i32) {
        let mut history = self.history.write();
        history.go(delta);
    }

    /// 返回上一页，等价于 go(-1)。
    pub fn back(&mut self) {
        let mut history = self.history.write();
        history.back();
    }

    /// 前进到下一页，等价于 go(1)。
    pub fn forward(&mut self) {
        let mut history = self.history.write();
        history.forward();
    }
}
