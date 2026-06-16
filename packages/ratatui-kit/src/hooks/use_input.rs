//! 输入层与事件 handler 钩子：取代旧的 `use_events` / `use_local_events`。
//!
//! - [`UseInputLayer::use_input_layer`]：声明一个输入层（模态独占等)，返回**同帧**句柄。
//! - [`UseEventHandler::use_event_handler`]：注册一个可消费的事件 handler。
//!
//! 两者均在组件函数体内经 `SystemContext` 当帧登记到 `InputRuntime`（取得守卫即用即弃)。
//! 因此必须在 **context-aware** 的 `Hooks` 上调用：函数组件（`#[component]`)由宏自动
//! `with_context_stack` 升级,开箱即用;**手写 `Component`** 需在 `update` 体内先
//! `let mut hooks = hooks.with_context_stack(updater.component_context_stack());`。

use std::{cell::Cell, rc::Rc};

use crossterm::event::Event;
use ratatui::layout::Rect;

use super::{Hook, Hooks};
use crate::{
    SystemContext, UseContext,
    input::{CurrentLayer, EventOptions, EventPriority, EventResult, EventScope, InputLayer},
};

mod private {
    pub trait Sealed {}
    impl Sealed for crate::hooks::Hooks<'_, '_> {}
}

pub trait UseInputLayer: private::Sealed {
    /// 声明一个输入层。`open=true` 时本帧参与分发;`blocks_lower=true` 时作为活跃栈顶截断更低层。
    ///
    /// 返回的 [`InputLayer`] 句柄仅**同帧**有效（每帧重新铸造)：可传给子树（`Modal` 的 `layer` prop)
    /// 或本组件 handler 的 [`EventScope::Layer`]。**禁止**存入 `use_state` 跨帧使用。
    fn use_input_layer(&mut self, open: bool, blocks_lower: bool) -> InputLayer;
}

pub trait UseEventHandler: private::Sealed {
    /// 注册一个事件 handler。`scope` 决定归属层、`priority` 决定同层投递顺序;闭包返回
    /// [`EventResult::Consumed`] 截断后续 handler。
    fn use_event_handler<F>(&mut self, scope: EventScope, priority: EventPriority, f: F)
    where
        F: FnMut(Event) -> EventResult + 'static;

    /// 带选项（如鼠标 `hit_test` 命中过滤)的注册。
    fn use_event_handler_with_options<F>(
        &mut self,
        scope: EventScope,
        priority: EventPriority,
        options: EventOptions,
        f: F,
    ) where
        F: FnMut(Event) -> EventResult + 'static;
}

/// `use_input_layer` 的占位 hook：无跨帧状态,仅占用一个稳定的 hook 顺序槽（满足 React 式顺序规则)。
struct UseInputLayerImpl;
impl Hook for UseInputLayerImpl {}

impl UseInputLayer for Hooks<'_, '_> {
    fn use_input_layer(&mut self, open: bool, blocks_lower: bool) -> InputLayer {
        self.use_hook(|| UseInputLayerImpl); // 占顺序槽
        // 当帧经 context 直接登记,取得守卫即用即弃。
        let mut sys = self.use_context_mut::<SystemContext>();
        sys.input.push_layer(open, blocks_lower)
    }
}

/// `use_event_handler` 的 hook：跨帧持有 `Rc<Cell<Rect>>`,在 `pre_component_draw` 回填**上一帧** area
/// 供鼠标 `hit_test`。闭包本身不跨帧保存（每帧经 `register_handler` 移交 `InputRuntime`,下帧重建)。
struct UseEventHandlerImpl {
    area: Rc<Cell<Rect>>,
}

impl Hook for UseEventHandlerImpl {
    fn pre_component_draw(&mut self, drawer: &mut crate::ComponentDrawer) {
        self.area.set(drawer.area);
    }
}

impl UseEventHandler for Hooks<'_, '_> {
    fn use_event_handler<F>(&mut self, scope: EventScope, priority: EventPriority, f: F)
    where
        F: FnMut(Event) -> EventResult + 'static,
    {
        self.use_event_handler_with_options(scope, priority, EventOptions::default(), f);
    }

    fn use_event_handler_with_options<F>(
        &mut self,
        scope: EventScope,
        priority: EventPriority,
        options: EventOptions,
        f: F,
    ) where
        F: FnMut(Event) -> EventResult + 'static,
    {
        // area 共享句柄:跨帧持有(use_hook),交给本帧的 HandlerEntry,pre_component_draw 回填。
        let area = {
            let hook = self.use_hook(|| UseEventHandlerImpl {
                area: Rc::new(Cell::new(Rect::default())),
            });
            hook.area.clone()
        };

        // 归属解析:Global → 无层;Layer(h) → 显式层;Current → context 最近 CurrentLayer,无则 root 层。
        let layer = match scope {
            EventScope::Global => None,
            EventScope::Layer(h) => Some(h.id),
            EventScope::Current => {
                let id = self
                    .try_use_context::<CurrentLayer>()
                    .map(|c| c.0)
                    .unwrap_or_else(|| self.use_context::<SystemContext>().input.root_layer());
                Some(id)
            }
        };

        // 当帧登记 handler,守卫即用即弃。
        let mut sys = self.use_context_mut::<SystemContext>();
        sys.input
            .register_handler(layer, priority, options, area, Box::new(f));
    }
}
