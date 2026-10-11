// 输入事件运行时：单一 raw 事件源 → 中央分发器 `InputRuntime`。
//
// 取代旧的「广播订阅」模型（每个 `use_events` 各自订阅、所有 handler 平等收到同一事件）。
// 核心能力：
// - **输入层栈**（`InputLayer` + `blocks_lower`）：模态层独占输入，背景层被截断。
// - **事件消费**（[`EventResult`]）：`Consumed` 截断后续 handler。
// - **优先级 / 作用域**（[`EventPriority`] / [`EventScope`]）：分层有序投递。
// - **每帧重建**：`begin_frame` 在每帧 update 开头清空层与 handler，组件在 update 期间重新登记，
//   因此关闭的弹窗 / 卸载的组件下一帧自动退出，无跨帧持久状态、无泄漏。
// - **指针冒泡**（设计见 `openspec/changes/add-pointer-events`）：鼠标命中链按**绘制序倒排**
//   （子先于父、后画兄弟先于先画兄弟），`Ignored` 向祖先 / 下层传播;`draw_seq` 与帧戳由
//   `pre_component_draw` 经 [`Stamp`] 回填，帧号不匹配当前帧视作未命中(防幽灵命中)。
// - **指针捕获路由**：hit_test handler 消费 `Down` 后，同按键的后续 `Drag`/`Up` 绕过命中
//   过滤独占直送捕获者(拖拽出区域事件不丢)。
// - **hover 全局 diff**：`Moved` 集中计算悬停集合,enter/leave 之外可跳过重绘。
//
// 运行时单线程渲染，故 handler 闭包不要求 `Send + Sync`。

use std::{cell::Cell, cell::RefCell, collections::HashMap, collections::HashSet, rc::Rc};

use crossterm::event::{Event, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::{Position, Rect};

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
#[doc(hidden)]
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
    // 命中区域取 [`Stamp::area`]，且要求本帧已绘制（帧戳匹配），否则视作未命中。
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

// 指针 handler 的跨帧稳定身份。由 hook 一次铸造、随 hook 存活（handler 表每帧重建，
// 捕获路由不能依赖帧内下标）。crate 内部类型,经 hooks 使用。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) struct PointerToken(u64);

// handler 所属组件的绘制快照：区域 + 帧号 + 帧内绘制序。
//
// 由 owning hook 在 `pre_component_draw` 经 `Rc<Cell<Stamp>>` 共享句柄回填（draw 期无 context,
// 帧号/绘制序取自线程局部 [`DRAW_CLOCK`]）。
// - `area`：鼠标 `hit_test` 的命中区域（上一帧区域,见设计文档「滞后一帧」）。
// - `frame`：回填时的帧号;`dispatch` 时不等于当前帧 → 组件本帧未绘制 → 视作未命中（防幽灵）。
// - `draw_seq`：帧内单调绘制序;鼠标分发按其**倒排** = 冒泡序（子先于父,后画者先）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Stamp {
    pub(crate) area: Rect,
    pub(crate) screen_area: Rect,
    pub(crate) delta_x: i32,
    pub(crate) delta_y: i32,
    pub(crate) frame: u64,
    pub(crate) draw_seq: u64,
}

impl Stamp {
    pub(crate) fn contains(self, position: Position) -> bool {
        self.frame == draw_clock_frame() && rect_contains(self.screen_area, position.x, position.y)
    }
    pub(crate) fn local_position(self, position: Position) -> Position {
        Position::new(
            (i32::from(position.x) - self.delta_x).clamp(0, i32::from(u16::MAX)) as u16,
            (i32::from(position.y) - self.delta_y).clamp(0, i32::from(u16::MAX)) as u16,
        )
    }
    pub(crate) fn same_geometry(self, other: Self) -> bool {
        self.area == other.area
            && self.screen_area == other.screen_area
            && self.delta_x == other.delta_x
            && self.delta_y == other.delta_y
    }
}

pub(crate) type StampHandle = Rc<Cell<Stamp>>;

// 绘制时钟：帧号 + 帧内绘制序。update/dispatch 与 draw 不在同一借用期,draw 期拿不到
// `SystemContext`,故用线程局部(运行时单线程渲染;单元测试各自线程内 begin_frame→dispatch 自洽)。
struct DrawClock {
    frame: u64,
    draw_seq: u64,
}

thread_local! {
    static DRAW_CLOCK: RefCell<DrawClock> = const { RefCell::new(DrawClock { frame: 0, draw_seq: 0 }) };
}

// `begin_frame` 调用:推进帧号、重置帧内绘制序。
fn draw_clock_begin_frame() {
    DRAW_CLOCK.with_borrow_mut(|c| {
        c.frame += 1;
        c.draw_seq = 0;
    });
}

// `pre_component_draw` 调用:推进绘制序并生成快照(调用顺序 = draw 遍历序 = 父先子后)。
pub(crate) fn draw_clock_stamp(area: Rect) -> Stamp {
    DRAW_CLOCK.with_borrow_mut(|c| {
        c.draw_seq += 1;
        Stamp {
            area,
            screen_area: area,
            delta_x: 0,
            delta_y: 0,
            frame: c.frame,
            draw_seq: c.draw_seq,
        }
    })
}

pub(crate) fn draw_clock_frame() -> u64 {
    DRAW_CLOCK.with_borrow(|c| c.frame)
}

// 本帧一个 handler 的登记。
struct HandlerEntry {
    // `None` = Global；`Some` = 归属层（`Current` 已解析为具体 `LayerId`）。
    layer: Option<LayerId>,
    priority: EventPriority,
    // 注册序，作为同层同优先级的稳定 tie-break（自顶向下，父先于子)。
    order: usize,
    options: EventOptions,
    // 组件绘制快照，由 owning hook 在 `pre_component_draw` 回填。
    stamp: StampHandle,
    // `Some` = 指针 handler（可被捕获路由命中、可登记 hover 追踪）。
    pointer: Option<PointerToken>,
    // 移动观察独立投递,消费只请求重绘,不能截断其它组件的 Leave。
    motion_observer: bool,
    cancel: Option<Box<dyn FnMut(MouseButton)>>,
    deactivate: Option<Box<dyn FnMut() -> bool>>,
    sync_motion: Option<Box<dyn FnMut(MouseEvent) -> bool>>,
    f: Box<dyn FnMut(Event) -> EventResult>,
}

// 中央事件运行时，挂在 `SystemContext` 上。层与 handler 表每帧重建;
// 捕获路由 / hover 集合 / 时钟为跨帧持久状态。
#[derive(Default)]
pub(crate) struct InputRuntime {
    layers: Vec<LayerEntry>,
    handlers: Vec<HandlerEntry>,
    next_layer_id: u64,
    root_layer: Option<LayerId>,
    // 指针捕获：按下时记录 (按钮 → 捕获者 token)，同按钮 Drag/Up 独占直送，Up 清除。
    // 组件卸载（当前帧无对应 handler）时自动失效。
    gestures: [Option<PointerToken>; 3],
    // 本帧 hover 追踪登记（由 `use_hover` 重建）。
    hover_trackers: Vec<(LayerId, PointerToken, StampHandle)>,
    // 悬停集合：指针当前所在的 tracker token 集（跨帧持久,由 Moved 增量维护）。
    hover_inside: HashSet<PointerToken>,
    // any-motion(1003h) 棘轮:存在过 hover/Move 追踪即置位,由渲染循环惰性执行升级。
    motion_requested: bool,
    motion_enabled: bool,
    last_pointer: Option<MouseEvent>,
}

// 铸造跨帧稳定的指针身份(hook 侧调用)。
// 不作为 InputRuntime 方法:铸造需要 context 借用,而 hooks 按调用序索引,
// 把一次 hook 访问拆成「读 + 写」两次 use_hook 会破坏顺序稳定性(见 use_pointer)。
// 全局唯一性由线程局部计数器保证(运行时单线程;测试各自线程内自洽)。
pub(crate) fn mint_pointer_token() -> PointerToken {
    thread_local! {
        static NEXT: Cell<u64> = const { Cell::new(0) };
    }
    NEXT.with(|c| {
        let next = c.get() + 1;
        c.set(next);
        PointerToken(next)
    })
}

// MouseButton → gestures 下标(Left/Right/Middle 各自独立捕获)。
fn button_index(button: MouseButton) -> usize {
    match button {
        MouseButton::Left => 0,
        MouseButton::Right => 1,
        MouseButton::Middle => 2,
    }
}

// Drag/Up 事件携带的按键。
fn pressed_button(kind: &MouseEventKind) -> Option<MouseButton> {
    match kind {
        MouseEventKind::Drag(b) | MouseEventKind::Up(b) => Some(*b),
        _ => None,
    }
}

// 点是否位于区域内。
pub(crate) fn rect_contains(area: Rect, column: u16, row: u16) -> bool {
    column >= area.x
        && column < area.x.saturating_add(area.width)
        && row >= area.y
        && row < area.y.saturating_add(area.height)
}

impl InputRuntime {
    // 每帧 update 开始时调用：清空上一帧的层与 handler，铸造并压入 root 层（`blocks_lower=false`），
    // 推进绘制时钟帧号。
    pub(crate) fn begin_frame(&mut self) {
        self.layers.clear();
        self.handlers.clear();
        self.hover_trackers.clear();
        let root = self.mint_layer_id();
        self.root_layer = Some(root);
        self.layers.push(LayerEntry {
            id: root,
            blocks_lower: false,
        });
        draw_clock_begin_frame();
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

    // 组件 update 期登记一个 handler。`layer=None` 表示全局 handler;
    // `pointer=Some` 标记指针 handler（参与捕获路由与 hover 追踪）。
    pub(crate) fn register_handler(
        &mut self,
        layer: Option<LayerId>,
        priority: EventPriority,
        options: EventOptions,
        stamp: StampHandle,
        pointer: Option<PointerToken>,
        f: Box<dyn FnMut(Event) -> EventResult>,
    ) {
        let order = self.handlers.len();
        self.handlers.push(HandlerEntry {
            layer,
            priority,
            order,
            options,
            stamp,
            pointer,
            motion_observer: false,
            cancel: None,
            deactivate: None,
            sync_motion: None,
            f,
        });
    }

    // 指针主 handler 的取消回调仅当前挂载帧有效,不保留已卸载组件的状态闭包。
    pub(crate) fn register_pointer_handler(
        &mut self,
        layer: LayerId,
        stamp: StampHandle,
        token: PointerToken,
        f: Box<dyn FnMut(Event) -> EventResult>,
        cancel: Box<dyn FnMut(MouseButton)>,
    ) {
        self.register_handler(
            Some(layer),
            EventPriority::Normal,
            EventOptions { hit_test: true },
            stamp,
            Some(token),
            f,
        );
        self.handlers.last_mut().unwrap().cancel = Some(cancel);
    }

    // 绘制结束才有完整的帧戳和输入层,避免隐藏/模态遮挡的捕获在恢复后复活。
    pub(crate) fn finish_frame(&mut self) -> bool {
        let mut repaint = false;
        let cut = self
            .layers
            .iter()
            .rposition(|layer| layer.blocks_lower)
            .unwrap_or(0);
        let frame = draw_clock_frame();
        for (index, button) in [MouseButton::Left, MouseButton::Right, MouseButton::Middle]
            .into_iter()
            .enumerate()
        {
            let Some(token) = self.gestures[index] else {
                continue;
            };
            let target = self
                .handlers
                .iter_mut()
                .find(|handler| handler.pointer == Some(token));
            let valid = target.as_ref().is_some_and(|handler| {
                handler.stamp.get().frame == frame
                    && !handler.stamp.get().screen_area.is_empty()
                    && handler
                        .layer
                        .is_some_and(|id| self.layers[cut..].iter().any(|layer| layer.id == id))
            });
            if !valid {
                self.gestures[index] = None;
                // 卸载后 Owner 中的 State 可能已释放,只取消仍挂载的目标。
                if let Some(cancel) = target.and_then(|handler| handler.cancel.as_mut()) {
                    cancel(button);
                    repaint = true;
                }
            }
        }
        for handler in self
            .handlers
            .iter_mut()
            .filter(|handler| handler.motion_observer)
        {
            let valid = handler.stamp.get().frame == frame
                && handler
                    .layer
                    .is_some_and(|id| self.layers[cut..].iter().any(|layer| layer.id == id));
            if !valid && let Some(deactivate) = handler.deactivate.as_mut() {
                repaint |= deactivate();
            } else if let Some(mouse) = self.last_pointer
                && let Some(sync) = handler.sync_motion.as_mut()
            {
                repaint |= sync(mouse);
            }
        }
        let new_inside: HashSet<_> = self
            .hover_trackers
            .iter()
            .filter(|(layer, _, stamp)| {
                stamp.get().frame == frame
                    && self.layers[cut..].iter().any(|entry| entry.id == *layer)
                    && self.last_pointer.is_some_and(|mouse| {
                        stamp.get().contains(Position::new(mouse.column, mouse.row))
                    })
            })
            .map(|(_, token, _)| *token)
            .collect();
        repaint |= new_inside != self.hover_inside;
        self.hover_inside = new_inside;
        repaint
    }

    pub(crate) fn register_motion_observer(
        &mut self,
        layer: LayerId,
        stamp: StampHandle,
        f: Box<dyn FnMut(Event) -> EventResult>,
        deactivate: Box<dyn FnMut() -> bool>,
        sync_motion: Box<dyn FnMut(MouseEvent) -> bool>,
    ) {
        self.request_motion();
        self.register_handler(
            Some(layer),
            EventPriority::Normal,
            EventOptions { hit_test: false },
            stamp,
            None,
            f,
        );
        let handler = self.handlers.last_mut().unwrap();
        handler.motion_observer = true;
        handler.deactivate = Some(deactivate);
        handler.sync_motion = Some(sync_motion);
    }

    // 登记 hover 追踪（`use_hover` 每帧调用）。同时置 any-motion 请求棘轮。
    pub(crate) fn track_hover(&mut self, layer: LayerId, token: PointerToken, stamp: StampHandle) {
        self.motion_requested = true;
        self.hover_trackers.push((layer, token, stamp));
    }

    // 指针当前是否悬停在该 token 组件区域内。
    pub(crate) fn hover_contains(&self, token: PointerToken) -> bool {
        self.hover_inside.contains(&token)
    }

    // 是否需要终端升级 any-motion 上报（存在过 hover 追踪且尚未升级）。
    pub(crate) fn needs_any_motion_upgrade(&self) -> bool {
        self.motion_requested && !self.motion_enabled
    }

    // 渲染循环执行 `enable_any_motion` 成功后回 mark（只升不降）。
    pub(crate) fn mark_any_motion_enabled(&mut self) {
        self.motion_enabled = true;
    }

    // 请求 any-motion 上报(存在 Move/Enter/Leave 追踪时调用;`use_hover` 经
    // `track_hover` 已内置,`use_pointer(track_move)` 显式请求)。
    pub(crate) fn request_motion(&mut self) {
        self.motion_requested = true;
    }

    // 在一次 render（update + draw）完整返回后、非借用期调用：把一个 raw 事件分发给本帧 handler。
    //
    // 返回 `needs_repaint`：`Moved` 无人消费且 hover 集合无变化时返回 `false`，
    // 渲染循环据此跳过本轮重绘（handler 写 State 会经 waker 立即唤醒,不依赖每事件无条件重绘）。
    //
    // 分发序：
    // 0. **捕获路由**：指针捕获生效时,同按钮 Drag/Up 独占直送捕获者（绕过命中过滤与排序）。
    // 1. **Global**：所有 `layer=None` handler，按 `(priority desc, order asc)`，遇 `Consumed` 终止全程。
    // 2. **层内**：活跃层（从栈顶向下遇首个 `blocks_lower` 截断)的 handler,
    //    按 `(层 z-order desc, priority desc, [鼠标: draw_seq desc], order asc)`——
    //    **z-order 第一键，不跨层比 priority**，遇 `Consumed` 早停。
    //    鼠标事件额外以**绘制序倒排**作 tie-break:子先于父、后画兄弟先于先画兄弟（冒泡）。
    pub(crate) fn dispatch(&mut self, event: Event) -> bool {
        let mut handlers = std::mem::take(&mut self.handlers);
        let repaint = self.dispatch_inner(&mut handlers, event);
        // 归还 handler 表:跳过重绘的路径下(Moved 无变化),本帧 handler 集必须继续
        // 服务后续事件;正常路径下帧的 begin_frame 会清空重建,归还无副作用。
        self.handlers = handlers;
        repaint
    }

    fn dispatch_inner(&mut self, handlers: &mut [HandlerEntry], event: Event) -> bool {
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

        let is_mouse = matches!(event, Event::Mouse(_));
        if let Event::Mouse(mouse) = event {
            self.last_pointer = Some(mouse);
        }
        let frame = draw_clock_frame();

        // hover 与消费无关:Global 提前截断也必须更新进入/离开状态。
        let hover_changed = if is_mouse
            && let Event::Mouse(m) = &event
            && matches!(m.kind, MouseEventKind::Moved)
        {
            let new_inside: HashSet<PointerToken> = self
                .hover_trackers
                .iter()
                .filter(|(layer, _, stamp)| {
                    let s = stamp.get();
                    active.contains_key(layer)
                        && s.frame == frame
                        && rect_contains(s.screen_area, m.column, m.row)
                })
                .map(|(_, token, _)| *token)
                .collect();
            let changed = new_inside != self.hover_inside;
            self.hover_inside = new_inside;
            changed
        } else {
            false
        };

        // 所有活跃组件先同步 Enter/Move/Leave,不受 Global 或另一观察者消费影响。
        let mut motion_changed = false;
        if matches!(event, Event::Mouse(m) if matches!(m.kind, MouseEventKind::Moved)) {
            for handler in handlers.iter_mut().filter(|handler| {
                handler.motion_observer
                    && handler
                        .layer
                        .is_some_and(|layer| active.contains_key(&layer))
            }) {
                motion_changed |= (handler.f)(event.clone()) == EventResult::Consumed;
            }
        }

        // Phase 0：指针捕获路由。捕获是对捕获者的投递承诺,先于 Global/层内链。
        if is_mouse
            && let Event::Mouse(m) = &event
            && let Some(button) = pressed_button(&m.kind)
        {
            let idx = button_index(button);
            if let Some(token) = self.gestures[idx] {
                // 当前帧仍注册且在活跃层内的捕获者才直送;否则捕获自动失效(组件卸载/层失活)。
                let target = (0..handlers.len()).find(|&i| {
                    handlers[i].pointer == Some(token)
                        && handlers[i].stamp.get().frame == frame
                        && handlers[i].layer.is_some_and(|l| active.contains_key(&l))
                });
                match target {
                    Some(i) => {
                        // 独占直送:绕过命中过滤(捕获语义),其他 handler 不参与本事件。
                        // 事件克隆后移交,Drag/Up 交互必然伴随状态变化,总是需要重绘。
                        let _ = (handlers[i].f)(event.clone());
                        if matches!(m.kind, MouseEventKind::Up(_)) {
                            self.gestures[idx] = None;
                        }
                        return true;
                    }
                    None => self.gestures[idx] = None,
                }
            }
        }

        // Phase 1：Global handler，按 (priority desc, order asc)。Consumed 即终止全程。
        let mut global_idx: Vec<usize> = (0..handlers.len())
            .filter(|&i| handlers[i].layer.is_none() && !handlers[i].motion_observer)
            .collect();
        global_idx.sort_by(|&a, &b| {
            handlers[b]
                .priority
                .cmp(&handlers[a].priority)
                .then(handlers[a].order.cmp(&handlers[b].order))
        });
        if Self::run_handlers(handlers, &global_idx, &event, frame).is_some() {
            return true;
        }

        // Phase 2：活跃层内。鼠标事件按绘制序倒排 tie-break（冒泡）;键盘维持注册序。
        let mut layer_idx: Vec<usize> = (0..handlers.len())
            .filter(|&i| {
                !handlers[i].motion_observer
                    && handlers[i].layer.is_some_and(|l| active.contains_key(&l))
            })
            .collect();
        if is_mouse {
            layer_idx.sort_by(|&a, &b| {
                let za = active[&handlers[a].layer.unwrap()];
                let zb = active[&handlers[b].layer.unwrap()];
                zb.cmp(&za) // z-order 降序：更靠栈顶的层先
                    .then(handlers[b].priority.cmp(&handlers[a].priority)) // priority 降序
                    .then(
                        handlers[b]
                            .stamp
                            .get()
                            .draw_seq
                            .cmp(&handlers[a].stamp.get().draw_seq),
                    ) // 绘制序倒排:后画者先
                    .then(handlers[a].order.cmp(&handlers[b].order)) // 注册序升序
            });
        } else {
            layer_idx.sort_by(|&a, &b| {
                let za = active[&handlers[a].layer.unwrap()];
                let zb = active[&handlers[b].layer.unwrap()];
                zb.cmp(&za)
                    .then(handlers[b].priority.cmp(&handlers[a].priority))
                    .then(handlers[a].order.cmp(&handlers[b].order))
            });
        }
        let consumed_by = Self::run_handlers(handlers, &layer_idx, &event, frame);
        let consumed = consumed_by.is_some();

        // 捕获记录:hit_test 指针 handler 消费了 Down → 隐式指针捕获该按键。
        if let (Some(i), Event::Mouse(m)) = (consumed_by, &event)
            && let MouseEventKind::Down(button) = m.kind
            && handlers[i].pointer.is_some()
            && handlers[i].options.hit_test
        {
            self.gestures[button_index(button)] = handlers[i].pointer;
        }

        if matches!(event, Event::Mouse(m) if matches!(m.kind, MouseEventKind::Moved)) {
            return consumed || hover_changed || motion_changed;
        }

        // handlers 由外层 dispatch 归还(见 dispatch 说明);下一帧 begin_frame 后由组件重建。
        true
    }

    // 按给定顺序依次调用 handler，遇 `Consumed` 早停并返回截断者下标。
    fn run_handlers(
        handlers: &mut [HandlerEntry],
        order: &[usize],
        event: &Event,
        frame: u64,
    ) -> Option<usize> {
        order
            .iter()
            .copied()
            .find(|&i| Self::call_handler(&mut handlers[i], event, frame) == EventResult::Consumed)
    }

    // 调用单个 handler，先做鼠标命中过滤（仅当 `hit_test` 且事件为鼠标事件)。
    // 区域外、或快照帧号不等于当前帧（组件本帧未绘制）视作未调用，返回 `Ignored` 让分发继续。
    fn call_handler(h: &mut HandlerEntry, event: &Event, frame: u64) -> EventResult {
        if h.options.hit_test
            && let Event::Mouse(m) = event
        {
            let s = h.stamp.get();
            if s.frame != frame || !rect_contains(s.screen_area, m.column, m.row) {
                return EventResult::Ignored;
            }
        }
        (h.f)(event.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
    use std::cell::RefCell;

    type Log = Rc<RefCell<Vec<&'static str>>>;

    fn key() -> Event {
        Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE))
    }

    fn mouse(kind: MouseEventKind, col: u16, row: u16) -> Event {
        Event::Mouse(MouseEvent {
            kind,
            column: col,
            row,
            modifiers: KeyModifiers::NONE,
        })
    }

    fn mouse_at(col: u16, row: u16) -> Event {
        mouse(MouseEventKind::Down(MouseButton::Left), col, row)
    }

    // 生成「本帧已绘制」的 stamp;连续调用获得递增 draw_seq,模拟 draw 遍历序。
    fn stamp(area: Rect) -> StampHandle {
        let h: StampHandle = Rc::new(Cell::new(Stamp::default()));
        h.set(draw_clock_stamp(area));
        h
    }

    // 从未绘制的 stamp(frame=0,当前帧必不匹配)。
    fn undrawn(area: Rect) -> StampHandle {
        Rc::new(Cell::new(Stamp {
            area,
            frame: 0,
            draw_seq: 0,
            ..Stamp::default()
        }))
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

    #[allow(clippy::too_many_arguments)]
    fn register(
        rt: &mut InputRuntime,
        layer: Option<LayerId>,
        options: EventOptions,
        stamp: StampHandle,
        token: Option<PointerToken>,
        log: &Log,
        tag: &'static str,
        result: EventResult,
    ) {
        rt.register_handler(
            layer,
            EventPriority::Normal,
            options,
            stamp,
            token,
            handler(log, tag, result),
        );
    }

    // ① blocks_lower 截断背景层
    #[test]
    fn blocks_lower_truncates_background() {
        let log: Log = Default::default();
        let mut rt = InputRuntime::default();
        rt.begin_frame();
        let root = rt.root_layer();
        register(
            &mut rt,
            Some(root),
            opts(false),
            stamp(Rect::new(0, 0, 100, 100)),
            None,
            &log,
            "bg",
            EventResult::Ignored,
        );
        let modal = rt.push_layer(true, true);
        register(
            &mut rt,
            Some(modal.id),
            opts(false),
            stamp(Rect::new(0, 0, 100, 100)),
            None,
            &log,
            "modal",
            EventResult::Ignored,
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
        register(
            &mut rt,
            Some(root),
            opts(false),
            stamp(Rect::new(0, 0, 100, 100)),
            None,
            &log,
            "root",
            EventResult::Ignored,
        );
        let l1 = rt.push_layer(true, true);
        register(
            &mut rt,
            Some(l1.id),
            opts(false),
            stamp(Rect::new(0, 0, 100, 100)),
            None,
            &log,
            "l1",
            EventResult::Ignored,
        );
        let l2 = rt.push_layer(true, true);
        register(
            &mut rt,
            Some(l2.id),
            opts(false),
            stamp(Rect::new(0, 0, 100, 100)),
            None,
            &log,
            "l2",
            EventResult::Ignored,
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
        register(
            &mut rt,
            Some(root),
            opts(false),
            stamp(Rect::new(0, 0, 100, 100)),
            None,
            &log,
            "root",
            EventResult::Ignored,
        );
        let modal = rt.push_layer(true, true);
        register(
            &mut rt,
            Some(modal.id),
            opts(false),
            stamp(Rect::new(0, 0, 100, 100)),
            None,
            &log,
            "modal",
            EventResult::Ignored,
        );
        let toast = rt.push_layer(true, false);
        register(
            &mut rt,
            Some(toast.id),
            opts(false),
            stamp(Rect::new(0, 0, 100, 100)),
            None,
            &log,
            "toast",
            EventResult::Ignored,
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
        register(
            &mut rt,
            Some(root),
            opts(false),
            stamp(Rect::new(0, 0, 100, 100)),
            None,
            &log,
            "first",
            EventResult::Consumed,
        );
        register(
            &mut rt,
            Some(root),
            opts(false),
            stamp(Rect::new(0, 0, 100, 100)),
            None,
            &log,
            "second",
            EventResult::Ignored,
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
        register(
            &mut rt,
            Some(root),
            opts(false),
            stamp(Rect::new(0, 0, 100, 100)),
            None,
            &log,
            "first",
            EventResult::Ignored,
        );
        register(
            &mut rt,
            Some(root),
            opts(false),
            stamp(Rect::new(0, 0, 100, 100)),
            None,
            &log,
            "second",
            EventResult::Ignored,
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
            stamp(Rect::new(0, 0, 100, 100)),
            None,
            handler(&log, "bg_high", EventResult::Ignored),
        );
        let top = rt.push_layer(true, false); // 非阻塞上层
        rt.register_handler(
            Some(top.id),
            EventPriority::Normal,
            opts(false),
            stamp(Rect::new(0, 0, 100, 100)),
            None,
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
        register(
            &mut rt,
            None,
            opts(false),
            stamp(Rect::new(0, 0, 100, 100)),
            None,
            &log,
            "global",
            EventResult::Consumed,
        );
        rt.register_handler(
            Some(root),
            EventPriority::High,
            opts(false),
            stamp(Rect::new(0, 0, 100, 100)),
            None,
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
        register(
            &mut rt,
            None,
            opts(false),
            stamp(Rect::new(0, 0, 100, 100)),
            None,
            &log,
            "global",
            EventResult::Ignored,
        );
        register(
            &mut rt,
            Some(root),
            opts(false),
            stamp(Rect::new(0, 0, 100, 100)),
            None,
            &log,
            "layer",
            EventResult::Ignored,
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
        register(
            &mut rt,
            Some(inactive.id),
            opts(false),
            stamp(Rect::new(0, 0, 100, 100)),
            None,
            &log,
            "inactive",
            EventResult::Ignored,
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
        register(
            &mut rt,
            Some(root),
            opts(true),
            stamp(Rect::new(0, 0, 10, 10)),
            None,
            &log,
            "hit",
            EventResult::Consumed,
        );
        rt.dispatch(mouse_at(50, 50)); // 区域外
        assert!(log.borrow().is_empty());

        rt.begin_frame(); // dispatch 已清 handlers,重建
        let root2 = rt.root_layer();
        register(
            &mut rt,
            Some(root2),
            opts(true),
            stamp(Rect::new(0, 0, 10, 10)),
            None,
            &log,
            "hit",
            EventResult::Consumed,
        );
        rt.dispatch(mouse_at(5, 5)); // 命中
        assert_eq!(*log.borrow(), ["hit"]);
    }

    // ⑨ 鼠标冒泡:绘制序倒排,子(后画)先于父(先画)
    #[test]
    fn mouse_child_before_parent() {
        let log: Log = Default::default();
        let mut rt = InputRuntime::default();
        rt.begin_frame();
        let root = rt.root_layer();
        // 父先注册、先画(大区域);子后注册、后画(小区域,嵌套在内)。
        register(
            &mut rt,
            Some(root),
            opts(true),
            stamp(Rect::new(0, 0, 100, 100)),
            None,
            &log,
            "parent",
            EventResult::Ignored,
        );
        register(
            &mut rt,
            Some(root),
            opts(true),
            stamp(Rect::new(10, 10, 5, 5)),
            None,
            &log,
            "child",
            EventResult::Ignored,
        );
        rt.dispatch(mouse_at(12, 12));
        assert_eq!(*log.borrow(), ["child", "parent"]);
    }

    // ⑩ 重叠兄弟:后画者(视觉在上)先收,注册序不参与
    #[test]
    fn mouse_overlapping_later_drawn_sibling_first() {
        let log: Log = Default::default();
        let mut rt = InputRuntime::default();
        rt.begin_frame();
        let root = rt.root_layer();
        // "early" 注册在前但先画;"late" 注册在后且后画。重叠区点击 → late 先。
        register(
            &mut rt,
            Some(root),
            opts(true),
            stamp(Rect::new(0, 0, 20, 5)),
            None,
            &log,
            "early",
            EventResult::Ignored,
        );
        register(
            &mut rt,
            Some(root),
            opts(true),
            stamp(Rect::new(10, 0, 20, 5)),
            None,
            &log,
            "late",
            EventResult::Ignored,
        );
        rt.dispatch(mouse_at(15, 2));
        assert_eq!(*log.borrow(), ["late", "early"]);
    }

    // ⑪ 键盘分发序不受鼠标冒泡改造影响(维持注册序)
    #[test]
    fn keyboard_order_unchanged_by_draw_seq() {
        let log: Log = Default::default();
        let mut rt = InputRuntime::default();
        rt.begin_frame();
        let root = rt.root_layer();
        register(
            &mut rt,
            Some(root),
            opts(true),
            stamp(Rect::new(0, 0, 100, 100)),
            None,
            &log,
            "first",
            EventResult::Ignored,
        );
        register(
            &mut rt,
            Some(root),
            opts(true),
            stamp(Rect::new(0, 0, 100, 100)),
            None,
            &log,
            "second",
            EventResult::Ignored,
        );
        rt.dispatch(key());
        assert_eq!(*log.borrow(), ["first", "second"]);
    }

    // ⑫ 帧戳防幽灵:组件本帧未绘制(stamp 是上一帧的)→ 旧区域不命中
    #[test]
    fn stale_frame_stamp_never_hits() {
        let log: Log = Default::default();
        let mut rt = InputRuntime::default();
        rt.begin_frame();
        let root = rt.root_layer();
        let stale = stamp(Rect::new(0, 0, 10, 10));
        register(
            &mut rt,
            Some(root),
            opts(true),
            stale.clone(),
            None,
            &log,
            "ghost",
            EventResult::Consumed,
        );
        rt.begin_frame(); // 时钟推进帧号,但组件未重画 → stamp 帧号落后
        let root2 = rt.root_layer();
        register(
            &mut rt,
            Some(root2),
            opts(true),
            stale,
            None,
            &log,
            "ghost",
            EventResult::Consumed,
        );
        rt.dispatch(mouse_at(5, 5));
        assert!(log.borrow().is_empty());
    }

    // ⑬ 从未绘制的组件(stamp 默认值)不命中
    #[test]
    fn never_drawn_stamp_never_hits() {
        let log: Log = Default::default();
        let mut rt = InputRuntime::default();
        rt.begin_frame();
        let root = rt.root_layer();
        register(
            &mut rt,
            Some(root),
            opts(true),
            undrawn(Rect::new(0, 0, 10, 10)),
            None,
            &log,
            "ghost",
            EventResult::Consumed,
        );
        rt.dispatch(mouse_at(5, 5));
        assert!(log.borrow().is_empty());
    }

    // ⑭ 遮罩组合:scrim(全屏) + 面板(子,消费)。点面板 → 仅面板;点空白 → 冒泡到 scrim。
    #[test]
    fn scrim_click_outside_panel_bubbles() {
        let log: Log = Default::default();
        let mut rt = InputRuntime::default();
        rt.begin_frame();
        let root = rt.root_layer();
        register(
            &mut rt,
            Some(root),
            opts(true),
            stamp(Rect::new(0, 0, 100, 100)),
            None,
            &log,
            "scrim",
            EventResult::Consumed,
        );
        register(
            &mut rt,
            Some(root),
            opts(true),
            stamp(Rect::new(40, 40, 20, 5)),
            None,
            &log,
            "panel",
            EventResult::Consumed,
        );
        rt.dispatch(mouse_at(50, 42)); // 面板内
        assert_eq!(*log.borrow(), ["panel"]);
        log.borrow_mut().clear();
        rt.begin_frame();
        let root2 = rt.root_layer();
        register(
            &mut rt,
            Some(root2),
            opts(true),
            stamp(Rect::new(0, 0, 100, 100)),
            None,
            &log,
            "scrim",
            EventResult::Consumed,
        );
        register(
            &mut rt,
            Some(root2),
            opts(true),
            stamp(Rect::new(40, 40, 20, 5)),
            None,
            &log,
            "panel",
            EventResult::Consumed,
        );
        rt.dispatch(mouse_at(5, 5)); // 面板外、scrim 内
        assert_eq!(*log.borrow(), ["scrim"]);
    }

    // ⑮ 指针捕获:Down 消费后,Drag/Up 出区域仍独占直送捕获者
    // 注意:真实循环中每个事件后有完整 update 重注册 handler,测试用 rebuild 模拟帧节奏。
    #[test]
    fn drag_capture_delivers_outside_area() {
        let log: Log = Default::default();
        let mut rt = InputRuntime::default();
        let token = mint_pointer_token();

        // 每帧重建:捕获者(token) + 旁观者,顺序保持捕获者先画、旁观者后画。
        let rebuild = |rt: &mut InputRuntime| {
            rt.begin_frame();
            let root = rt.root_layer();
            register(
                rt,
                Some(root),
                opts(true),
                stamp(Rect::new(0, 0, 10, 10)),
                Some(token),
                &log,
                "captor",
                EventResult::Consumed,
            );
            register(
                rt,
                Some(root),
                opts(true),
                stamp(Rect::new(40, 40, 20, 20)),
                None,
                &log,
                "bystander",
                EventResult::Consumed,
            );
        };

        rebuild(&mut rt);
        rt.dispatch(mouse_at(5, 5)); // Down(Left) 在捕获者区域内
        assert_eq!(*log.borrow(), ["captor"]);
        log.borrow_mut().clear();

        rebuild(&mut rt);
        // Drag 移入 bystander 区域:仍独占直送捕获者(出其区域不丢事件)。
        rt.dispatch(mouse(MouseEventKind::Drag(MouseButton::Left), 50, 50));
        assert_eq!(*log.borrow(), ["captor"]);
        log.borrow_mut().clear();

        rebuild(&mut rt);
        // Up 同样直送;随后捕获解除。
        rt.dispatch(mouse(MouseEventKind::Up(MouseButton::Left), 50, 50));
        assert_eq!(*log.borrow(), ["captor"]);
        log.borrow_mut().clear();

        rebuild(&mut rt);
        // 捕获解除后:同位置 Drag 不再直送,按命中分发 → bystander。
        rt.dispatch(mouse(MouseEventKind::Drag(MouseButton::Left), 50, 50));
        assert_eq!(*log.borrow(), ["bystander"]);
    }

    // ⑯ 捕获者卸载:token 无对应 handler → 捕获自动失效,事件正常分发
    #[test]
    fn unmounted_captor_releases_capture() {
        let log: Log = Default::default();
        let mut rt = InputRuntime::default();
        rt.begin_frame();
        let root = rt.root_layer();
        let token = mint_pointer_token();
        register(
            &mut rt,
            Some(root),
            opts(true),
            stamp(Rect::new(0, 0, 10, 10)),
            Some(token),
            &log,
            "captor",
            EventResult::Consumed,
        );
        rt.dispatch(mouse_at(5, 5));
        assert_eq!(*log.borrow(), ["captor"]);
        log.borrow_mut().clear();

        // 下一帧捕获者组件卸载,只剩 bystander。
        rt.begin_frame();
        let root2 = rt.root_layer();
        register(
            &mut rt,
            Some(root2),
            opts(true),
            stamp(Rect::new(0, 0, 10, 10)),
            None,
            &log,
            "bystander",
            EventResult::Consumed,
        );
        rt.dispatch(mouse(MouseEventKind::Up(MouseButton::Left), 5, 5));
        assert_eq!(*log.borrow(), ["bystander"]);
    }

    #[test]
    fn hidden_captor_is_cancelled_after_draw_and_never_revives() {
        let mut runtime = InputRuntime::default();
        let token = mint_pointer_token();
        let cancelled = Rc::new(Cell::new(0));
        let calls = Rc::new(Cell::new(0));
        let rebuild = |runtime: &mut InputRuntime, drawn: bool, blocked: bool| {
            runtime.begin_frame();
            let root = runtime.root_layer();
            let cancel_count = cancelled.clone();
            let call_count = calls.clone();
            runtime.register_pointer_handler(
                root,
                if drawn {
                    stamp(Rect::new(0, 0, 10, 10))
                } else {
                    undrawn(Rect::new(0, 0, 10, 10))
                },
                token,
                Box::new(move |_| {
                    call_count.set(call_count.get() + 1);
                    EventResult::Consumed
                }),
                Box::new(move |_| cancel_count.set(cancel_count.get() + 1)),
            );
            runtime.push_layer(blocked, true);
            runtime.finish_frame();
        };
        for (drawn, blocked) in [(false, false), (true, true)] {
            rebuild(&mut runtime, true, false);
            runtime.dispatch(mouse_at(5, 5));
            rebuild(&mut runtime, drawn, blocked);
            assert!(runtime.gestures[0].is_none());
            let before = calls.get();
            // 恢复时未发生任何释放事件,旧捕获仍必须保持取消。
            rebuild(&mut runtime, true, false);
            runtime.dispatch(mouse(MouseEventKind::Drag(MouseButton::Left), 50, 50));
            assert_eq!(calls.get(), before);
        }
        assert_eq!(cancelled.get(), 2);
    }

    #[test]
    fn unmounted_pointer_releases_capture_without_calling_disposed_callback() {
        let mut runtime = InputRuntime::default();
        runtime.begin_frame();
        let cancelled = Rc::new(Cell::new(false));
        let flag = cancelled.clone();
        runtime.register_pointer_handler(
            runtime.root_layer(),
            stamp(Rect::new(0, 0, 10, 10)),
            mint_pointer_token(),
            Box::new(|_| EventResult::Consumed),
            Box::new(move |_| flag.set(true)),
        );
        runtime.dispatch(mouse_at(5, 5));
        runtime.begin_frame();
        runtime.finish_frame();
        assert!(!cancelled.get());
        assert!(runtime.gestures[0].is_none());
    }

    // ⑰ 多按键独立捕获:Left 由 A 捕获、Right 由 B 捕获,互不干扰
    #[test]
    fn two_buttons_capture_independently() {
        let log: Log = Default::default();
        let mut rt = InputRuntime::default();
        let token_a = mint_pointer_token();
        let token_b = mint_pointer_token();

        let rebuild = |rt: &mut InputRuntime| {
            rt.begin_frame();
            let root = rt.root_layer();
            register(
                rt,
                Some(root),
                opts(true),
                stamp(Rect::new(0, 0, 10, 10)),
                Some(token_a),
                &log,
                "a",
                EventResult::Consumed,
            );
            register(
                rt,
                Some(root),
                opts(true),
                stamp(Rect::new(40, 40, 10, 10)),
                Some(token_b),
                &log,
                "b",
                EventResult::Consumed,
            );
        };

        rebuild(&mut rt);
        rt.dispatch(mouse(MouseEventKind::Down(MouseButton::Left), 5, 5));
        rebuild(&mut rt);
        rt.dispatch(mouse(MouseEventKind::Down(MouseButton::Right), 45, 45));
        log.borrow_mut().clear();

        rebuild(&mut rt);
        rt.dispatch(mouse(MouseEventKind::Drag(MouseButton::Left), 45, 45));
        assert_eq!(*log.borrow(), ["a"]);
        log.borrow_mut().clear();

        rebuild(&mut rt);
        rt.dispatch(mouse(MouseEventKind::Drag(MouseButton::Right), 5, 5));
        assert_eq!(*log.borrow(), ["b"]);
    }

    // ⑱ hover:移入/移出更新集合,区域内重复移动无变化(可跳过重绘)
    #[test]
    fn hover_enter_leave_updates_state() {
        let mut rt = InputRuntime::default();
        rt.begin_frame();
        let token = mint_pointer_token();
        let tracker_area = stamp(Rect::new(0, 0, 10, 10));
        rt.track_hover(rt.root_layer(), token, tracker_area);

        let repaint = rt.dispatch(mouse(MouseEventKind::Moved, 5, 5));
        assert!(repaint); // enter → 变化
        assert!(rt.hover_contains(token));

        let repaint = rt.dispatch(mouse(MouseEventKind::Moved, 50, 50));
        assert!(repaint); // leave → 变化
        assert!(!rt.hover_contains(token));

        let repaint = rt.dispatch(mouse(MouseEventKind::Moved, 6, 6));
        assert!(repaint); // 再次 enter
        assert!(rt.hover_contains(token));

        // 区域内持续移动:无变化、无人消费 → 跳过重绘。
        let repaint = rt.dispatch(mouse(MouseEventKind::Moved, 7, 7));
        assert!(!repaint);
        assert!(rt.hover_contains(token));
    }

    // 模态层阻断背景 hover;模态关闭后背景恢复。
    #[test]
    fn blocking_modal_layer_clears_background_hover() {
        let mut rt = InputRuntime::default();
        let background = mint_pointer_token();
        let modal = mint_pointer_token();
        let area = Rect::new(0, 0, 10, 10);
        rt.begin_frame();
        rt.track_hover(rt.root_layer(), background, stamp(area));
        rt.dispatch(mouse(MouseEventKind::Moved, 5, 5));
        assert!(rt.hover_contains(background));

        rt.begin_frame();
        rt.track_hover(rt.root_layer(), background, stamp(area));
        let layer = rt.push_layer(true, true);
        rt.track_hover(layer.id, modal, stamp(area));
        assert!(rt.dispatch(mouse(MouseEventKind::Moved, 5, 5)));
        assert!(!rt.hover_contains(background));
        assert!(rt.hover_contains(modal));

        rt.begin_frame();
        rt.track_hover(rt.root_layer(), background, stamp(area));
        let layer = rt.push_layer(false, true);
        rt.track_hover(layer.id, modal, stamp(area));
        assert!(rt.dispatch(mouse(MouseEventKind::Moved, 5, 5)));
        assert!(rt.hover_contains(background));
        assert!(!rt.hover_contains(modal));
    }

    #[test]
    fn global_consumer_does_not_prevent_hover_updates() {
        let mut rt = InputRuntime::default();
        rt.begin_frame();
        let token = mint_pointer_token();
        rt.track_hover(rt.root_layer(), token, stamp(Rect::new(0, 0, 10, 10)));
        let log: Log = Default::default();
        register(
            &mut rt,
            None,
            opts(false),
            stamp(Rect::default()),
            None,
            &log,
            "global",
            EventResult::Consumed,
        );
        assert!(rt.dispatch(mouse(MouseEventKind::Moved, 5, 5)));
        assert!(rt.hover_contains(token));
        assert!(rt.dispatch(mouse(MouseEventKind::Moved, 50, 50)));
        assert!(!rt.hover_contains(token));
    }

    // ⑲ 无 hover 追踪、无人消费的 Moved → 跳过重绘
    #[test]
    fn moved_without_trackers_skips_repaint() {
        let mut rt = InputRuntime::default();
        rt.begin_frame();
        assert!(!rt.dispatch(mouse(MouseEventKind::Moved, 5, 5)));
    }

    // ⑳ Moved 被消费 → 仍重绘
    #[test]
    fn moved_consumed_repaints() {
        let log: Log = Default::default();
        let mut rt = InputRuntime::default();
        rt.begin_frame();
        let root = rt.root_layer();
        register(
            &mut rt,
            Some(root),
            opts(true),
            stamp(Rect::new(0, 0, 10, 10)),
            None,
            &log,
            "observer",
            EventResult::Consumed,
        );
        assert!(rt.dispatch(mouse(MouseEventKind::Moved, 5, 5)));
    }

    // ㉑ any-motion 棘轮:存在 hover 追踪才请求升级,升级后不再请求
    #[test]
    fn any_motion_upgrade_ratchet() {
        let mut rt = InputRuntime::default();
        rt.begin_frame();
        assert!(!rt.needs_any_motion_upgrade());
        let token = mint_pointer_token();
        rt.track_hover(rt.root_layer(), token, stamp(Rect::new(0, 0, 10, 10)));
        assert!(rt.needs_any_motion_upgrade());
        rt.mark_any_motion_enabled();
        assert!(!rt.needs_any_motion_upgrade());
    }

    // ㉒b 回归:Moved 跳过重绘后,handler 表不得被清空 —— 后续事件(如点击)必须仍能命中。
    // 真实场景:指针悬停微动(skip render)后点击,首次点击不应被吞。
    #[test]
    fn moved_skip_repaint_keeps_handler_registry() {
        let log: Log = Default::default();
        let mut rt = InputRuntime::default();
        rt.begin_frame();
        let root = rt.root_layer();
        let token = mint_pointer_token();
        // 模拟真实 Button:忽略 Moved、消费其余命中事件。
        let click_log = log.clone();
        rt.register_handler(
            Some(root),
            EventPriority::Normal,
            opts(true),
            stamp(Rect::new(0, 0, 10, 10)),
            Some(token),
            Box::new(move |event| {
                if matches!(
                    event,
                    Event::Mouse(MouseEvent {
                        kind: MouseEventKind::Moved,
                        ..
                    })
                ) {
                    return EventResult::Ignored;
                }
                click_log.borrow_mut().push("btn");
                EventResult::Consumed
            }),
        );

        // 无 hover 追踪、无消费的 Moved → needs_repaint=false(渲染循环会跳过重绘)。
        assert!(!rt.dispatch(mouse(MouseEventKind::Moved, 5, 5)));

        // 不重建帧(渲染被跳过):紧随其后的 Down 必须仍命中 handler。
        rt.dispatch(mouse_at(5, 5));
        assert_eq!(*log.borrow(), ["btn"]);
    }

    // ㉒ 非鼠标事件始终重绘(维持既有「每事件必重绘」行为)
    #[test]
    fn non_mouse_event_always_repaints() {
        let mut rt = InputRuntime::default();
        rt.begin_frame();
        assert!(rt.dispatch(key()));
    }
}
