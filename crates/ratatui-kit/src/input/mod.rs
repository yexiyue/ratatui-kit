// 输入事件运行时：单一 raw 事件源 → 中央分发器 `InputRuntime`。
//
// 取代旧的「广播订阅」模型（每个 `use_events` 各自订阅、所有 handler 平等收到同一事件）。
// 核心能力：
// - **输入层栈**（`InputLayer` + `blocks_lower`）：模态层独占输入，背景层被截断。
// - **事件消费**（[`EventResult`]）：`Consumed` 截断后续 handler。
// - **优先级 / 作用域**（[`EventPriority`] / [`EventScope`]）：分层有序投递。
// - **每帧重建**：`begin_frame` 在每帧 update 开头清空层与 handler，组件在 update 期间重新登记，
//   因此关闭的弹窗 / 卸载的组件下一帧自动退出，无跨帧持久状态、无泄漏。
//
// 运行时单线程渲染，故 handler 闭包不要求 `Send + Sync`。

use std::{cell::Cell, collections::HashMap, rc::Rc};

use crossterm::event::Event;
use ratatui::layout::Rect;

// handler 处理事件后的结果。`Default = Ignored`（让事件继续向后传)。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum EventResult {
    // 未消费，继续投递给后续 handler。
    #[default]
    Ignored,
    // 已消费，停止向后续 handler 传播。
    Consumed,
}

// 事件投递优先级。同一层内 `High` 先于 `Normal` 先于 `Low`。
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum EventPriority {
    Low = 0,
    #[default]
    Normal = 1,
    High = 2,
}

// 输入层身份。每帧由 [`InputRuntime`] 单调铸造，**跨帧不复用、不稳定**。
//
// [`InputLayer`] 句柄仅在**同一帧**内由父组件传给子组件用于 [`EventScope::Layer`] 显式归属；
// **禁止**存入 `use_state` 跨帧使用（下一帧该 id 已不在层栈，对应 handler 会静默失聪）。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct LayerId(u64);

// 用户持有的输入层句柄（`Copy`）。由 `use_input_layer` 返回。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct InputLayer {
    pub(crate) id: LayerId,
}

// handler 的归属作用域。
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EventScope {
    // 继承 context 栈中最近的 [`CurrentLayer`]，无则归属 root 层。
    // 用于背景组件与 `Modal` 子树内的 handler。
    Current,
    // 显式绑定到给定层。用于 handler 注册在 `Modal` 父组件、而 `Modal` 经该句柄开层的场景。
    Layer(InputLayer),
    // 真全局：不受任何 `blocks_lower` 截断（如 Resize、全局帮助键）。
    Global,
}

// handler 登记选项。
#[derive(Clone, Copy, Default)]
pub struct EventOptions {
    // `true` 时鼠标事件仅在 handler 所属组件区域内命中才调用（键盘等事件不受影响)。
    // 复刻旧 `use_local_events` 的命中过滤。
    pub hit_test: bool,
}

// context 注入项：子树据此把 [`EventScope::Current`] 解析到所属层。
// 由 `Modal` / `use_input_layer` 经 `update_children(.., Some(Context::owned(CurrentLayer(id))))` 注入。
#[derive(Clone, Copy)]
pub(crate) struct CurrentLayer(pub(crate) LayerId);

// 本帧一个输入层的登记。注册序（在 `layers` 中的下标）= update 自顶向下 = z 序：下标越大越靠上。
struct LayerEntry {
    id: LayerId,
    // `true` 时作为活跃栈顶会截断其下所有非 `Global` handler（模态独占）。
    blocks_lower: bool,
}

// 本帧一个 handler 的登记。
struct HandlerEntry {
    // `None` = Global；`Some` = 归属层（`Current` 已解析为具体 `LayerId`）。
    layer: Option<LayerId>,
    priority: EventPriority,
    // 注册序，作为同层同优先级的稳定 tie-break（自顶向下，父先于子)。
    order: usize,
    options: EventOptions,
    // handler 所属组件区域，由 owning hook 在 `pre_component_draw` 经共享句柄回填（上一帧尺寸)。
    area: Rc<Cell<Rect>>,
    f: Box<dyn FnMut(Event) -> EventResult>,
}

// 中央事件运行时，挂在 `SystemContext` 上。每帧重建层与 handler 表。
#[derive(Default)]
pub(crate) struct InputRuntime {
    layers: Vec<LayerEntry>,
    handlers: Vec<HandlerEntry>,
    next_layer_id: u64,
    root_layer: Option<LayerId>,
}

impl InputRuntime {
    // 每帧 update 开始时调用：清空上一帧的层与 handler，铸造并压入 root 层（`blocks_lower=false`）。
    pub(crate) fn begin_frame(&mut self) {
        self.layers.clear();
        self.handlers.clear();
        let root = self.mint_layer_id();
        self.root_layer = Some(root);
        self.layers.push(LayerEntry {
            id: root,
            blocks_lower: false,
        });
    }

    // 当前帧 root 层 id。`begin_frame` 后必然存在。
    pub(crate) fn root_layer(&self) -> LayerId {
        self.root_layer
            .expect("`begin_frame` was not called before `root_layer`")
    }

    fn mint_layer_id(&mut self) -> LayerId {
        let id = LayerId(self.next_layer_id);
        self.next_layer_id = self.next_layer_id.wrapping_add(1);
        id
    }

    // 组件 update 期登记一个输入层，返回句柄。
    //
    // `open=false` 时仍铸造并返回 id（供同帧 `use_event_handler(Layer(h))` 绑定），
    // 但**不入** `layers` 栈 → 绑定到它的 handler 因不在活跃集而静默跳过。
    pub(crate) fn push_layer(&mut self, open: bool, blocks_lower: bool) -> InputLayer {
        let id = self.mint_layer_id();
        if open {
            self.layers.push(LayerEntry { id, blocks_lower });
        }
        InputLayer { id }
    }

    // 组件 update 期登记一个 handler。`layer=None` 表示全局 handler。
    pub(crate) fn register_handler(
        &mut self,
        layer: Option<LayerId>,
        priority: EventPriority,
        options: EventOptions,
        area: Rc<Cell<Rect>>,
        f: Box<dyn FnMut(Event) -> EventResult>,
    ) {
        let order = self.handlers.len();
        self.handlers.push(HandlerEntry {
            layer,
            priority,
            order,
            options,
            area,
            f,
        });
    }

    // 在一次 render（update + draw）完整返回后、非借用期调用：把一个 raw 事件分发给本帧 handler。
    //
    // 两个 phase：
    // 1. **Global**：所有 `layer=None` handler，按 `(priority desc, order asc)`，遇 `Consumed` 终止全程。
    // 2. **层内**：活跃层（从栈顶向下遇首个 `blocks_lower` 截断)的 handler，
    //    按 `(层 z-order desc, priority desc, order asc)`——**z-order 第一键，不跨层比 priority**，遇 `Consumed` 早停。
    pub(crate) fn dispatch(&mut self, event: Event) {
        // 活跃层集：从栈顶（末尾)向下，遇首个 blocks_lower=true 截断（含该层)。
        let cut = self
            .layers
            .iter()
            .rposition(|e| e.blocks_lower)
            .unwrap_or(0);
        // 活跃层 id -> z 序（在 layers 中的下标，越大越靠上)。
        let active: HashMap<LayerId, usize> = self.layers[cut..]
            .iter()
            .enumerate()
            .map(|(off, e)| (e.id, cut + off))
            .collect();

        // mem::take 取出 handlers 遍历，消除「持 &mut self.handlers 调闭包」的自借用脆弱性。
        let mut handlers = std::mem::take(&mut self.handlers);

        // Phase 1：Global handler，按 (priority desc, order asc)。Consumed 即终止全程。
        let mut global_idx: Vec<usize> = (0..handlers.len())
            .filter(|&i| handlers[i].layer.is_none())
            .collect();
        global_idx.sort_by(|&a, &b| {
            handlers[b]
                .priority
                .cmp(&handlers[a].priority)
                .then(handlers[a].order.cmp(&handlers[b].order))
        });
        if Self::run_handlers(&mut handlers, &global_idx, &event) {
            return;
        }

        // Phase 2：活跃层内，按 (z-order desc, priority desc, order asc)。
        let mut layer_idx: Vec<usize> = (0..handlers.len())
            .filter(|&i| handlers[i].layer.is_some_and(|l| active.contains_key(&l)))
            .collect();
        layer_idx.sort_by(|&a, &b| {
            let za = active[&handlers[a].layer.unwrap()];
            let zb = active[&handlers[b].layer.unwrap()];
            zb.cmp(&za) // z-order 降序：更靠栈顶的层先
                .then(handlers[b].priority.cmp(&handlers[a].priority)) // priority 降序
                .then(handlers[a].order.cmp(&handlers[b].order)) // 注册序升序
        });
        Self::run_handlers(&mut handlers, &layer_idx, &event);
        // handlers 在此 drop（即弃)；下一帧 begin_frame 后由组件重建。
    }

    // 按给定顺序依次调用 handler，遇 `Consumed` 早停并返回 `true`
    // （供 Phase 1 决定是否截断 Phase 2）。
    fn run_handlers(handlers: &mut [HandlerEntry], order: &[usize], event: &Event) -> bool {
        for &i in order {
            if Self::call_handler(&mut handlers[i], event) == EventResult::Consumed {
                return true;
            }
        }
        false
    }

    // 调用单个 handler，先做鼠标命中过滤（仅当 `hit_test` 且事件为鼠标事件)。
    // 区域外视作未调用，返回 `Ignored` 让分发继续下一个候选。
    fn call_handler(h: &mut HandlerEntry, event: &Event) -> EventResult {
        if h.options.hit_test
            && let Event::Mouse(m) = event
        {
            let a = h.area.get();
            let hit = m.column >= a.x
                && m.column < a.x.saturating_add(a.width)
                && m.row >= a.y
                && m.row < a.y.saturating_add(a.height);
            if !hit {
                return EventResult::Ignored;
            }
        }
        (h.f)(event.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{
        KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    };
    use std::cell::RefCell;

    type Log = Rc<RefCell<Vec<&'static str>>>;

    fn key() -> Event {
        Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE))
    }

    fn mouse_at(col: u16, row: u16) -> Event {
        Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: col,
            row,
            modifiers: KeyModifiers::NONE,
        })
    }

    fn full_area() -> Rc<Cell<Rect>> {
        Rc::new(Cell::new(Rect::new(0, 0, 100, 100)))
    }

    fn handler(
        log: &Log,
        tag: &'static str,
        result: EventResult,
    ) -> Box<dyn FnMut(Event) -> EventResult> {
        let log = log.clone();
        Box::new(move |_| {
            log.borrow_mut().push(tag);
            result
        })
    }

    fn opts(hit_test: bool) -> EventOptions {
        EventOptions { hit_test }
    }

    // ① blocks_lower 截断背景层
    #[test]
    fn blocks_lower_truncates_background() {
        let log: Log = Default::default();
        let mut rt = InputRuntime::default();
        rt.begin_frame();
        let root = rt.root_layer();
        rt.register_handler(
            Some(root),
            EventPriority::Normal,
            opts(false),
            full_area(),
            handler(&log, "bg", EventResult::Ignored),
        );
        let modal = rt.push_layer(true, true);
        rt.register_handler(
            Some(modal.id),
            EventPriority::Normal,
            opts(false),
            full_area(),
            handler(&log, "modal", EventResult::Ignored),
        );
        rt.dispatch(key());
        assert_eq!(*log.borrow(), ["modal"]);
    }

    // ② 嵌套 blocks_lower 只激活最顶层
    #[test]
    fn nested_blocks_lower_activates_only_top() {
        let log: Log = Default::default();
        let mut rt = InputRuntime::default();
        rt.begin_frame();
        let root = rt.root_layer();
        rt.register_handler(
            Some(root),
            EventPriority::Normal,
            opts(false),
            full_area(),
            handler(&log, "root", EventResult::Ignored),
        );
        let l1 = rt.push_layer(true, true);
        rt.register_handler(
            Some(l1.id),
            EventPriority::Normal,
            opts(false),
            full_area(),
            handler(&log, "l1", EventResult::Ignored),
        );
        let l2 = rt.push_layer(true, true);
        rt.register_handler(
            Some(l2.id),
            EventPriority::Normal,
            opts(false),
            full_area(),
            handler(&log, "l2", EventResult::Ignored),
        );
        rt.dispatch(key());
        assert_eq!(*log.borrow(), ["l2"]);
    }

    // ②b 顶部非阻塞层仍活跃,但首个 blocks_lower 以下失活
    #[test]
    fn non_blocking_layers_above_blocker_remain_active() {
        let log: Log = Default::default();
        let mut rt = InputRuntime::default();
        rt.begin_frame();
        let root = rt.root_layer();
        rt.register_handler(
            Some(root),
            EventPriority::Normal,
            opts(false),
            full_area(),
            handler(&log, "root", EventResult::Ignored),
        );
        let modal = rt.push_layer(true, true);
        rt.register_handler(
            Some(modal.id),
            EventPriority::Normal,
            opts(false),
            full_area(),
            handler(&log, "modal", EventResult::Ignored),
        );
        let toast = rt.push_layer(true, false);
        rt.register_handler(
            Some(toast.id),
            EventPriority::Normal,
            opts(false),
            full_area(),
            handler(&log, "toast", EventResult::Ignored),
        );

        rt.dispatch(key());
        assert_eq!(*log.borrow(), ["toast", "modal"]);
    }

    // ③ Consumed 截断后续 handler
    #[test]
    fn consumed_stops_subsequent() {
        let log: Log = Default::default();
        let mut rt = InputRuntime::default();
        rt.begin_frame();
        let root = rt.root_layer();
        rt.register_handler(
            Some(root),
            EventPriority::Normal,
            opts(false),
            full_area(),
            handler(&log, "first", EventResult::Consumed),
        );
        rt.register_handler(
            Some(root),
            EventPriority::Normal,
            opts(false),
            full_area(),
            handler(&log, "second", EventResult::Ignored),
        );
        rt.dispatch(key());
        assert_eq!(*log.borrow(), ["first"]);
    }

    // ④ Ignored 继续传播（同层按注册序)
    #[test]
    fn ignored_continues_propagation() {
        let log: Log = Default::default();
        let mut rt = InputRuntime::default();
        rt.begin_frame();
        let root = rt.root_layer();
        rt.register_handler(
            Some(root),
            EventPriority::Normal,
            opts(false),
            full_area(),
            handler(&log, "first", EventResult::Ignored),
        );
        rt.register_handler(
            Some(root),
            EventPriority::Normal,
            opts(false),
            full_area(),
            handler(&log, "second", EventResult::Ignored),
        );
        rt.dispatch(key());
        assert_eq!(*log.borrow(), ["first", "second"]);
    }

    // ⑤ 层 z-order 优先于 priority（下层 High 不抢上层 Normal)
    #[test]
    fn layer_z_order_beats_priority() {
        let log: Log = Default::default();
        let mut rt = InputRuntime::default();
        rt.begin_frame();
        let root = rt.root_layer();
        rt.register_handler(
            Some(root),
            EventPriority::High,
            opts(false),
            full_area(),
            handler(&log, "bg_high", EventResult::Ignored),
        );
        let top = rt.push_layer(true, false); // 非阻塞上层
        rt.register_handler(
            Some(top.id),
            EventPriority::Normal,
            opts(false),
            full_area(),
            handler(&log, "top_normal", EventResult::Ignored),
        );
        rt.dispatch(key());
        assert_eq!(*log.borrow(), ["top_normal", "bg_high"]);
    }

    // ⑥ Global 独立 phase:先跑且可 Consumed 截断
    #[test]
    fn global_phase_first_and_can_consume() {
        let log: Log = Default::default();
        let mut rt = InputRuntime::default();
        rt.begin_frame();
        let root = rt.root_layer();
        rt.register_handler(
            None,
            EventPriority::Normal,
            opts(false),
            full_area(),
            handler(&log, "global", EventResult::Consumed),
        );
        rt.register_handler(
            Some(root),
            EventPriority::High,
            opts(false),
            full_area(),
            handler(&log, "layer", EventResult::Ignored),
        );
        rt.dispatch(key());
        assert_eq!(*log.borrow(), ["global"]);
    }

    // ⑥b Global Ignored 不截断（observer 语义)
    #[test]
    fn global_ignored_does_not_truncate() {
        let log: Log = Default::default();
        let mut rt = InputRuntime::default();
        rt.begin_frame();
        let root = rt.root_layer();
        rt.register_handler(
            None,
            EventPriority::Normal,
            opts(false),
            full_area(),
            handler(&log, "global", EventResult::Ignored),
        );
        rt.register_handler(
            Some(root),
            EventPriority::Normal,
            opts(false),
            full_area(),
            handler(&log, "layer", EventResult::Ignored),
        );
        rt.dispatch(key());
        assert_eq!(*log.borrow(), ["global", "layer"]);
    }

    // ⑦ handler 绑定 inactive/missing 层时不调用
    #[test]
    fn inactive_layer_handler_skipped() {
        let log: Log = Default::default();
        let mut rt = InputRuntime::default();
        rt.begin_frame();
        let inactive = rt.push_layer(false, true); // open=false → 不入栈
        rt.register_handler(
            Some(inactive.id),
            EventPriority::Normal,
            opts(false),
            full_area(),
            handler(&log, "inactive", EventResult::Ignored),
        );
        rt.dispatch(key());
        assert!(log.borrow().is_empty());
    }

    // ⑧ hit_test 区域外跳过、区域内命中
    #[test]
    fn hit_test_skips_outside_area() {
        let log: Log = Default::default();
        let mut rt = InputRuntime::default();
        rt.begin_frame();
        let root = rt.root_layer();
        let area = Rc::new(Cell::new(Rect::new(0, 0, 10, 10)));
        rt.register_handler(
            Some(root),
            EventPriority::Normal,
            opts(true),
            area,
            handler(&log, "hit", EventResult::Consumed),
        );
        rt.dispatch(mouse_at(50, 50)); // 区域外
        assert!(log.borrow().is_empty());

        rt.begin_frame(); // dispatch 已清 handlers,重建
        let root2 = rt.root_layer();
        let area2 = Rc::new(Cell::new(Rect::new(0, 0, 10, 10)));
        rt.register_handler(
            Some(root2),
            EventPriority::Normal,
            opts(true),
            area2,
            handler(&log, "hit", EventResult::Consumed),
        );
        rt.dispatch(mouse_at(5, 5)); // 命中
        assert_eq!(*log.borrow(), ["hit"]);
    }
}
