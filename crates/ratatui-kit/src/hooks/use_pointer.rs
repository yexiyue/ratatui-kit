// 语义化指针交互 hooks:点击 / 悬停 / 拖拽 / 完整指针流。
//
// 分层(设计见 `openspec/changes/add-pointer-events/design.md` D5/D8):
// - `use_click`:Down+Up 同目标合成 [`Click`](含双击/三击计数),处理后默认 `Consumed`。
// - `use_hover`:返回指针是否在本组件区域内(集中 diff,仅 enter/leave 变化反映到渲染)。
// - `use_drag`:基于指针捕获的 Start/Move/End 三态(拖出区域事件不丢)。
// - `use_pointer`:完整 [`PointerEvent`] 流,消费/放行由回调返回 [`EventResult`] 自控。
//
// 底层语义(冒泡排序、帧戳、捕获路由)由 `InputRuntime` 提供;点击合成等纯局部
// 状态机在 hook 层。所有指针 hook **要求**应用入口启用鼠标捕获
// (`element!(App).fullscreen_with(RunOptions {{ mouse: true, ..Default::default() }})`),
// debug 构建下未启用即注册会断言并给出指引,release 下静默不投递。

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};

use crossterm::event::{Event, KeyModifiers, MouseButton, MouseEventKind};
use ratatui::layout::{Position, Rect};

use super::use_input::new_stamp_handle;
use crate::{
    Hook, Hooks, SystemContext, UseContext,
    input::{
        CurrentLayer, EventOptions, EventPriority, EventResult, InputLayer, LayerId, PointerToken,
        Stamp, StampHandle, rect_contains,
    },
};

// 双击判定窗口:窗口期内在邻近位置(≤ 2 格)再次按下,计数递增,上限 3。
const DOUBLE_CLICK_WINDOW: Duration = Duration::from_millis(300);
const CLICK_POSITION_TOLERANCE: u16 = 2;
const CLICK_COUNT_MAX: u8 = 3;

// ---- 公开事件类型 ----

/// 指针事件种类。`Down`/`Up`/`Drag`/`Click` 携带相关按键;`Scroll*` 四向;
/// `Move`/`Enter`/`Leave` 需要 [`PointerOptions::track_move`]。
/// `Cancel` 保留最后已知位置和修饰键,仅清理交互,不应提交释放动作。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PointerKind {
    /// 按键按下。
    Down(MouseButton),
    /// 按键释放。
    Up(MouseButton),
    /// 点击(Down+Up 同目标合成)。`count` 为窗口期内连击计数(1=单击,2=双击,3=三击)。
    Click { button: MouseButton, count: u8 },
    /// 按住移动(经由指针捕获,拖出区域仍会收到)。
    Drag(MouseButton),
    /// 捕获因隐藏、卸载或输入层失活而取消,不合成点击。
    Cancel(MouseButton),
    /// 无按键移动(区域内)。
    Move,
    /// 指针进入组件区域。
    Enter,
    /// 指针离开组件区域。
    Leave,
    /// 向上滚动。
    ScrollUp,
    /// 向下滚动。
    ScrollDown,
    /// 向左滚动。
    ScrollLeft,
    /// 向右滚动。
    ScrollRight,
}

/// 指针事件:`position` 与 `area` 共用组件绘制坐标,可直接交给组件widget命中。
/// `screen_position` 保留终端绝对坐标;滚动容器统一处理内容坐标映射与视口裁剪。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PointerEvent {
    /// 事件种类。
    pub kind: PointerKind,
    /// 事件位置(组件绘制单元格坐标,与area同一坐标空间)。
    pub position: Position,
    /// 终端屏幕绝对坐标,不受滚动容器的内容坐标变换影响。
    pub screen_position: Position,
    /// 修饰键。
    pub modifiers: KeyModifiers,
    /// 本组件上一帧绘制区域。
    pub area: Rect,
}

/// 按键集合过滤(位标志式组合,如 `MouseButtons::LEFT.union(MouseButtons::RIGHT)`)。
/// 默认仅左键。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MouseButtons(u8);

impl Default for MouseButtons {
    fn default() -> Self {
        Self::LEFT
    }
}

impl MouseButtons {
    /// 左键(默认值)。
    pub const LEFT: Self = Self(0b001);
    /// 右键。
    pub const RIGHT: Self = Self(0b010);
    /// 中键。
    pub const MIDDLE: Self = Self(0b100);
    /// 全部按键。
    pub const ALL: Self = Self(0b111);

    /// 并集。
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// 是否包含 `other` 的任一按键。
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

pub(crate) fn buttons_of(button: MouseButton) -> MouseButtons {
    match button {
        MouseButton::Left => MouseButtons::LEFT,
        MouseButton::Right => MouseButtons::RIGHT,
        MouseButton::Middle => MouseButtons::MIDDLE,
    }
}

/// `use_pointer` 的注册选项。
#[derive(Clone, Copy, Debug, Default)]
pub struct PointerOptions {
    /// 参与的按键(Down/Up/Drag/Click 过滤;Scroll 不受影响)。默认仅左键。
    pub buttons: MouseButtons,
    /// 显式归属输入层,None使用最近Current层。句柄仅同帧有效。
    pub layer: Option<InputLayer>,
    /// 是否接收 `Move`/`Enter`/`Leave`。开启后框架会向终端请求全量移动上报
    /// (any-motion,惰性升级、只升不降),移动事件频率较高,按需开启。
    /// 移动观察独立投递:返回 Consumed 请求重绘,不截断其它组件的进入/离开通知。
    pub track_move: bool,
}

/// `use_click` 的合成点击回调入参。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Click {
    /// 窗口期内连击计数:1=单击,2=双击,3=三击(上限)。
    pub count: u8,
    /// 按键。
    pub button: MouseButton,
    /// 释放(合成判定)位置。
    pub position: Position,
    /// 修饰键。
    pub modifiers: KeyModifiers,
}

/// `use_drag` 的拖拽阶段。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DragPhase {
    /// 按下开始(`from == to == 按下点`)。
    Start,
    /// 按住移动。
    Move,
    /// 释放结束。
    End,
    /// 组件隐藏、卸载或输入层失活,拖拽取消。
    Cancel,
}

/// `use_drag` 的回调入参。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DragUpdate {
    /// 阶段。
    pub phase: DragPhase,
    /// 按键(当前实现固定左键)。
    pub button: MouseButton,
    /// 起点(上一事件位置;`Start` 时为按下点)。
    pub from: Position,
    /// 终点(本事件位置)。
    pub to: Position,
    /// 修饰键。
    pub modifiers: KeyModifiers,
}

// ---- 共享状态 ----

// 点击合成状态机(`use_click` 与 `use_pointer` 复用):
// Down 挂起待配对;Up 匹配且落在区域内时产出计数,并把该 Down 的信息记为
// 「上一次完成点击」——下一个 Down 在窗口期 + 邻近位置内命中它则计数递增(双击/三击)。
pub(crate) struct ClickSynthesizer {
    // 未配对的当前 Down(按键,时间,位置)。
    pending: Option<(MouseButton, Instant, u16, u16)>,
    // 上一次完成点击的 Down 信息(连击窗口判定基准)。
    last_click: Option<(MouseButton, Instant, u16, u16)>,
    count: u8,
}

impl ClickSynthesizer {
    pub(crate) fn new() -> Self {
        Self {
            pending: None,
            last_click: None,
            count: 0,
        }
    }

    pub(crate) fn on_down(&mut self, button: MouseButton, column: u16, row: u16) {
        let now = Instant::now();
        let consecutive = self.last_click.is_some_and(|(b, t, c, r)| {
            b == button
                && now.duration_since(t) <= DOUBLE_CLICK_WINDOW
                && column.abs_diff(c) <= CLICK_POSITION_TOLERANCE
                && row.abs_diff(r) <= CLICK_POSITION_TOLERANCE
        });
        self.count = if consecutive {
            (self.count + 1).min(CLICK_COUNT_MAX)
        } else {
            1
        };
        self.pending = Some((button, now, column, row));
    }

    fn cancel(&mut self) {
        self.pending = None;
        self.last_click = None;
        self.count = 0;
    }

    // Up 匹配 pending 时产出计数;`inside` 为释放位置是否仍在组件区域内
    // (跨目标 / 拖出后的释放不合成,连击链随之中断)。
    pub(crate) fn on_up(&mut self, button: MouseButton, inside: bool) -> Option<u8> {
        let matched = self.pending.take().filter(|(b, ..)| *b == button);
        match matched {
            Some(info) if inside => {
                self.last_click = Some(info);
                Some(self.count)
            }
            _ => None,
        }
    }
}

// 指针 hook 公共底座:stamp 句柄(pre_component_draw 回填)+ 惰性铸造的跨帧 token。
pub(crate) struct PointerHookBase {
    stamp: StampHandle,
    token: Cell<Option<PointerToken>>,
}

pub(crate) trait AsPointerBase {
    fn as_base(&self) -> &PointerHookBase;
    fn new_pointer_hook() -> Self;
}

impl PointerHookBase {
    pub(crate) fn new() -> Self {
        Self {
            stamp: new_stamp_handle(),
            token: Cell::new(None),
        }
    }

    // 取(或首次铸造)指针身份:stamp 句柄 + 跨帧 token。
    // 关键约束:调用方每帧必须**恰好一次** use_hook 拿到本 hook(身份与共享状态同源),
    // hooks 按调用序索引,多调一次即错位(会命中另一个实例)。
    pub(crate) fn identity(&self) -> (StampHandle, PointerToken) {
        let token = self.token.get().unwrap_or_else(|| {
            let token = crate::input::mint_pointer_token();
            self.token.set(Some(token));
            token
        });
        (self.stamp.clone(), token)
    }

    fn pre_draw(&self, drawer: &crate::ComponentDrawer) {
        self.stamp.set(drawer.input_stamp());
    }
}

/// 指针交互可用性:应用入口启用鼠标捕获才为真。
///
/// 组件内置的 `use_pointer` 注册前应先查它——未启用捕获时 `use_pointer` 会 debug 断言。
/// `mouse_enabled` 在进程生命周期内不变,故「条件注册」的分支跨帧稳定,不违反 hook 顺序。
///
/// 注意:**手写 `Component::update` 必须先 `hooks.with_context_stack(updater.component_context_stack())`**
/// 再调本函数——未接 context stack 时读不到 `SystemContext`,本函数返回 false,指针功能被静默跳过。
pub(crate) fn pointer_enabled(hooks: &Hooks<'_, '_>) -> bool {
    match hooks.try_use_context::<SystemContext>() {
        Some(sys) => sys.mouse_enabled(),
        None => {
            // 读不到 SystemContext 通常是手写 Component 忘了接 context stack
            //(fn 组件由宏自动接)。debug 断言给出指引;release 仍静默返回 false。
            debug_assert!(
                false,
                "pointer hooks called on Hooks without a context stack; hand-written \
                 Component::update must call \
                 hooks.with_context_stack(updater.component_context_stack()) first"
            );
            false
        }
    }
}

// `EventScope::Current` 的层解析(context 最近 CurrentLayer,无则 root)。
fn current_layer_id(hooks: &Hooks<'_, '_>) -> LayerId {
    hooks
        .try_use_context::<CurrentLayer>()
        .map(|c| c.0)
        .unwrap_or_else(|| hooks.use_context::<SystemContext>().input.root_layer())
}

// 指针注册前的哑坑断言:未启用鼠标捕获时给出指引(debug 生效;release 静默不投递)。
fn debug_assert_mouse_enabled(sys: &SystemContext) {
    debug_assert!(
        sys.mouse_enabled(),
        "pointer handler registered but mouse capture is not enabled;\n  \
         enable it at the app entry: \
         element!(App).fullscreen_with(RunOptions {{ mouse: true, ..Default::default() }})"
    );
}

// ---- hook 实现 ----

mod private {
    pub trait Sealed {}
    impl Sealed for crate::hooks::Hooks<'_, '_> {}
}

/// 点击交互:Down+Up 同目标合成 [`Click`],双击窗口内计数递增。
///
/// 命中处理后消费事件(不向父级冒泡);需要放行语义请用 [`UsePointer::use_pointer`]。
pub trait UseClick: private::Sealed {
    fn use_click<F>(&mut self, f: F)
    where
        F: FnMut(Click) + 'static;
}

struct UseClickImpl {
    base: PointerHookBase,
    synth: Rc<RefCell<ClickSynthesizer>>,
}

impl AsPointerBase for UseClickImpl {
    fn as_base(&self) -> &PointerHookBase {
        &self.base
    }
    fn new_pointer_hook() -> Self {
        Self {
            base: PointerHookBase::new(),
            synth: Rc::new(RefCell::new(ClickSynthesizer::new())),
        }
    }
}

impl Hook for UseClickImpl {
    fn pre_component_draw(&mut self, drawer: &mut crate::ComponentDrawer) {
        self.base.pre_draw(drawer);
    }
}

impl UseClick for Hooks<'_, '_> {
    fn use_click<F>(&mut self, mut f: F)
    where
        F: FnMut(Click) + 'static,
    {
        let hook = self.use_hook(UseClickImpl::new_pointer_hook);
        let (stamp, token) = hook.as_base().identity();
        let synth = hook.synth.clone();
        let hit_area = stamp.clone();
        let cancel_synth = synth.clone();
        let layer = current_layer_id(self);
        let mut sys = self.use_context_mut::<SystemContext>();
        debug_assert_mouse_enabled(&sys);
        sys.input.register_pointer_handler(
            layer,
            stamp,
            token,
            Box::new(move |event| {
                let Event::Mouse(m) = event else {
                    return EventResult::Ignored;
                };
                match m.kind {
                    // 消费 Down:隐式指针捕获 → 后续 Drag/Up 即使出区域也直送本 handler。
                    MouseEventKind::Down(button) => {
                        synth.borrow_mut().on_down(button, m.column, m.row);
                        EventResult::Consumed
                    }
                    MouseEventKind::Up(button) => {
                        let inside = hit_area.get().contains(Position::new(m.column, m.row));
                        if let Some(count) = synth.borrow_mut().on_up(button, inside) {
                            f(Click {
                                count,
                                button,
                                position: Position::new(m.column, m.row),
                                modifiers: m.modifiers,
                            });
                        }
                        EventResult::Consumed
                    }
                    _ => EventResult::Ignored,
                }
            }),
            Box::new(move |_| cancel_synth.borrow_mut().cancel()),
        );
    }
}

/// 悬停状态:返回指针当前是否在本组件区域内(上一帧绘制区域)。
///
/// 存在悬停追踪时框架自动向终端请求全量移动上报(any-motion,惰性升级、只升不降);
/// 区域内部的移动不产生额外重绘。
pub trait UseHover: private::Sealed {
    fn use_hover(&mut self) -> bool;
}

struct UseHoverImpl {
    base: PointerHookBase,
}

impl AsPointerBase for UseHoverImpl {
    fn as_base(&self) -> &PointerHookBase {
        &self.base
    }
    fn new_pointer_hook() -> Self {
        Self {
            base: PointerHookBase::new(),
        }
    }
}

impl Hook for UseHoverImpl {
    fn pre_component_draw(&mut self, drawer: &mut crate::ComponentDrawer) {
        self.base.pre_draw(drawer);
    }
}

impl UseHover for Hooks<'_, '_> {
    fn use_hover(&mut self) -> bool {
        let (stamp, token) = self
            .use_hook(UseHoverImpl::new_pointer_hook)
            .as_base()
            .identity();
        let layer = current_layer_id(self);
        let mut sys = self.use_context_mut::<SystemContext>();
        debug_assert_mouse_enabled(&sys);
        sys.input.track_hover(layer, token, stamp);
        sys.input.hover_contains(token)
    }
}

/// 拖拽交互:基于指针捕获的三态回调(拖出区域事件不丢)。当前实现固定左键。
pub trait UseDrag: private::Sealed {
    fn use_drag<F>(&mut self, f: F)
    where
        F: FnMut(DragUpdate) + 'static;
}

struct UseDragImpl {
    base: PointerHookBase,
    active: Rc<Cell<Option<(MouseButton, Position)>>>,
}

impl AsPointerBase for UseDragImpl {
    fn as_base(&self) -> &PointerHookBase {
        &self.base
    }
    fn new_pointer_hook() -> Self {
        Self {
            base: PointerHookBase::new(),
            active: Rc::new(Cell::new(None)),
        }
    }
}

impl Hook for UseDragImpl {
    fn pre_component_draw(&mut self, drawer: &mut crate::ComponentDrawer) {
        self.base.pre_draw(drawer);
    }
}

impl UseDrag for Hooks<'_, '_> {
    fn use_drag<F>(&mut self, f: F)
    where
        F: FnMut(DragUpdate) + 'static,
    {
        let f = Rc::new(RefCell::new(f));
        let cancel_f = f.clone();
        let hook = self.use_hook(UseDragImpl::new_pointer_hook);
        let (stamp, token) = hook.as_base().identity();
        let active = hook.active.clone();
        let cancel_active = active.clone();
        let layer = current_layer_id(self);
        let mut sys = self.use_context_mut::<SystemContext>();
        debug_assert_mouse_enabled(&sys);
        sys.input.register_pointer_handler(
            layer,
            stamp,
            token,
            Box::new(move |event| {
                let Event::Mouse(m) = event else {
                    return EventResult::Ignored;
                };
                let to = Position::new(m.column, m.row);
                match m.kind {
                    MouseEventKind::Down(button) if button == MouseButton::Left => {
                        active.set(Some((button, to)));
                        (f.borrow_mut())(DragUpdate {
                            phase: DragPhase::Start,
                            button,
                            from: to,
                            to,
                            modifiers: m.modifiers,
                        });
                        EventResult::Consumed
                    }
                    MouseEventKind::Drag(MouseButton::Left) => {
                        if let Some((b, last)) = active.get() {
                            active.set(Some((b, to)));
                            (f.borrow_mut())(DragUpdate {
                                phase: DragPhase::Move,
                                button: b,
                                from: last,
                                to,
                                modifiers: m.modifiers,
                            });
                            EventResult::Consumed
                        } else {
                            EventResult::Ignored
                        }
                    }
                    MouseEventKind::Up(MouseButton::Left) => {
                        if let Some((b, last)) = active.take() {
                            (f.borrow_mut())(DragUpdate {
                                phase: DragPhase::End,
                                button: b,
                                from: last,
                                to,
                                modifiers: m.modifiers,
                            });
                            EventResult::Consumed
                        } else {
                            EventResult::Ignored
                        }
                    }
                    _ => EventResult::Ignored,
                }
            }),
            Box::new(move |button| {
                if let Some((_, position)) = cancel_active.take() {
                    (cancel_f.borrow_mut())(DragUpdate {
                        phase: DragPhase::Cancel,
                        button,
                        from: position,
                        to: position,
                        modifiers: KeyModifiers::NONE,
                    });
                }
            }),
        );
    }
}

/// 完整指针流:接收 [`PointerEvent`],消费/放行由回调返回 [`EventResult`] 自控。
///
/// 注:`Click` 合成依赖 Down 的配对 Up 能到达本 handler —— 对 Down 返回 `Ignored`
/// 不会形成指针捕获,Up 可能落在其他组件上(不合成 Click)。需要可靠的点击语义用
/// [`UseClick::use_click`]。
pub trait UsePointer: private::Sealed {
    fn use_pointer<F>(&mut self, options: PointerOptions, f: F)
    where
        F: FnMut(PointerEvent) -> EventResult + 'static;
}

struct UsePointerImpl {
    base: PointerHookBase,
    synth: Rc<RefCell<ClickSynthesizer>>,
    last_inside: Rc<Cell<bool>>,
    last_event: Rc<Cell<(Position, KeyModifiers)>>,
    last_geometry: Rc<Cell<Option<Stamp>>>,
}

impl AsPointerBase for UsePointerImpl {
    fn as_base(&self) -> &PointerHookBase {
        &self.base
    }
    fn new_pointer_hook() -> Self {
        Self {
            base: PointerHookBase::new(),
            synth: Rc::new(RefCell::new(ClickSynthesizer::new())),
            last_inside: Rc::new(Cell::new(false)),
            last_event: Rc::new(Cell::new((Position::new(0, 0), KeyModifiers::NONE))),
            last_geometry: Rc::new(Cell::new(None)),
        }
    }
}

impl Hook for UsePointerImpl {
    fn pre_component_draw(&mut self, drawer: &mut crate::ComponentDrawer) {
        self.base.pre_draw(drawer);
    }
}

impl UsePointer for Hooks<'_, '_> {
    fn use_pointer<F>(&mut self, options: PointerOptions, f: F)
    where
        F: FnMut(PointerEvent) -> EventResult + 'static,
    {
        register_pointer(self, options, false, f);
    }
}

// 受控悬停回调仅状态变化时消费,可每帧核对内容变化（条目offset等）,无需伪造全局Move。
pub(crate) fn use_pointer_with_layout_sync<F>(
    hooks: &mut Hooks<'_, '_>,
    options: PointerOptions,
    f: F,
) where
    F: FnMut(PointerEvent) -> EventResult + 'static,
{
    register_pointer(hooks, options, true, f);
}

fn register_pointer<F>(
    hooks: &mut Hooks<'_, '_>,
    options: PointerOptions,
    sync_on_render: bool,
    f: F,
) where
    F: FnMut(PointerEvent) -> EventResult + 'static,
{
    // 主 handler 与 track_move 观察者共享同一回调。
    let f = Rc::new(RefCell::new(f));
    let hook = hooks.use_hook(UsePointerImpl::new_pointer_hook);
    let (stamp, token) = hook.as_base().identity();
    let synth = hook.synth.clone();
    let last_inside = hook.last_inside.clone();
    let last_event = hook.last_event.clone();
    let last_geometry = hook.last_geometry.clone();
    let layer = options
        .layer
        .map(|layer| layer.id)
        .unwrap_or_else(|| current_layer_id(hooks));
    let buttons = options.buttons;

    // 主 handler:命中过滤(Down/Up/Drag/Click/Scroll;Moved 交由 track_move 观察者)。
    {
        let hit_area = stamp.clone();
        let cancel_area = stamp.clone();
        let cancel_synth = synth.clone();
        let cancel_inside = last_inside.clone();
        let cancel_f = f.clone();
        let cancel_event = last_event.clone();
        let pointer_event = last_event.clone();
        let f = f.clone();
        let mut sys = hooks.use_context_mut::<SystemContext>();
        debug_assert_mouse_enabled(&sys);
        sys.input.register_pointer_handler(
            layer,
            stamp.clone(),
            token,
            Box::new(move |event| {
                let Event::Mouse(m) = event else {
                    return EventResult::Ignored;
                };
                pointer_event.set((Position::new(m.column, m.row), m.modifiers));
                let stamp = hit_area.get();
                let area = stamp.area;
                let screen_position = Position::new(m.column, m.row);
                let ev = |kind| PointerEvent {
                    kind,
                    position: stamp.local_position(screen_position),
                    screen_position,
                    modifiers: m.modifiers,
                    area,
                };
                match m.kind {
                    MouseEventKind::Down(button) if buttons.contains(buttons_of(button)) => {
                        synth.borrow_mut().on_down(button, m.column, m.row);
                        (f.borrow_mut())(ev(PointerKind::Down(button)))
                    }
                    MouseEventKind::Up(button) if buttons.contains(buttons_of(button)) => {
                        // hit_test 命中到达时必在区域内;捕获路由到达时可能在外。
                        let inside = stamp.contains(screen_position);
                        let mut result = (f.borrow_mut())(ev(PointerKind::Up(button)));
                        if let Some(count) = synth.borrow_mut().on_up(button, inside)
                            && (f.borrow_mut())(ev(PointerKind::Click { button, count }))
                                == EventResult::Consumed
                        {
                            result = EventResult::Consumed;
                        }
                        result
                    }
                    MouseEventKind::Drag(button) if buttons.contains(buttons_of(button)) => {
                        (f.borrow_mut())(ev(PointerKind::Drag(button)))
                    }
                    MouseEventKind::ScrollUp => (f.borrow_mut())(ev(PointerKind::ScrollUp)),
                    MouseEventKind::ScrollDown => (f.borrow_mut())(ev(PointerKind::ScrollDown)),
                    MouseEventKind::ScrollLeft => (f.borrow_mut())(ev(PointerKind::ScrollLeft)),
                    MouseEventKind::ScrollRight => (f.borrow_mut())(ev(PointerKind::ScrollRight)),
                    // Moved 由 track_move 观察者统一处理(避免双投递)。
                    _ => EventResult::Ignored,
                }
            }),
            Box::new(move |button| {
                cancel_synth.borrow_mut().cancel();
                cancel_inside.set(false);
                let stamp = cancel_area.get();
                let area = stamp.area;
                let (position, modifiers) = cancel_event.get();
                (cancel_f.borrow_mut())(PointerEvent {
                    kind: PointerKind::Cancel(button),
                    position: stamp.local_position(position),
                    screen_position: position,
                    modifiers,
                    area,
                });
            }),
        );
    }

    // Move/Enter/Leave 观察者:无命中过滤(需看到区域外的移动才能判 Leave),
    // 只处理 Moved,其余一律放行;请求 any-motion 上报。
    if options.track_move {
        let motion_stamp = stamp.clone();
        let motion_inside = last_inside.clone();
        let motion_event = last_event.clone();
        let motion_geometry = last_geometry.clone();
        let motion_f = f.clone();
        // 同一处理器用于真实Moved和绘制后几何同步;后者只在几何发生变化时发Move。
        let motion = Rc::new(RefCell::new(
            move |m: crossterm::event::MouseEvent, sync: bool| {
                motion_event.set((Position::new(m.column, m.row), m.modifiers));
                let stamp = motion_stamp.get();
                let changed_geometry = motion_geometry
                    .replace(Some(stamp))
                    .is_none_or(|previous| !previous.same_geometry(stamp));
                let screen_position = Position::new(m.column, m.row);
                let inside = stamp.contains(screen_position);
                let kind = match (motion_inside.replace(inside), inside) {
                    (false, true) => PointerKind::Enter,
                    (true, false) => PointerKind::Leave,
                    (true, true) if !sync || changed_geometry || sync_on_render => {
                        PointerKind::Move
                    }
                    _ => return false,
                };
                (motion_f.borrow_mut())(PointerEvent {
                    kind,
                    position: stamp.local_position(screen_position),
                    screen_position,
                    modifiers: m.modifiers,
                    area: stamp.area,
                }) == EventResult::Consumed
            },
        ));
        let dispatch_motion = motion.clone();
        let leave_f = f.clone();
        let leave_stamp = stamp.clone();
        let leave_event = last_event.clone();
        let mut sys = hooks.use_context_mut::<SystemContext>();
        sys.input.register_motion_observer(
            layer,
            stamp,
            Box::new(move |event| {
                if let Event::Mouse(mouse) = event
                    && matches!(mouse.kind, MouseEventKind::Moved)
                {
                    if (dispatch_motion.borrow_mut())(mouse, false) {
                        EventResult::Consumed
                    } else {
                        EventResult::Ignored
                    }
                } else {
                    EventResult::Ignored
                }
            }),
            Box::new(move || {
                if last_inside.replace(false) {
                    let stamp = leave_stamp.get();
                    let (position, modifiers) = leave_event.get();
                    (leave_f.borrow_mut())(PointerEvent {
                        kind: PointerKind::Leave,
                        position: stamp.local_position(position),
                        screen_position: position,
                        modifiers,
                        area: stamp.area,
                    });
                    true
                } else {
                    false
                }
            }),
            Box::new(move |mouse| (motion.borrow_mut())(mouse, true)),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AnyElement, ComponentDrawer, ElementRepr, Hooks,
        components::{Text, View},
        input::EventOptions,
        render::tree::Tree,
        terminal::UpdaterTerminal,
    };
    use crossterm::event::MouseEventKind;
    use ratatui::layout::Direction;
    use ratatui_kit_macros::{Props, component};

    use crate::{
        UseEventHandler, UseState,
        input::EventScope,
        mouse_test::{Mounted, TestLog, log_of},
    };

    // ---- 测试组件 ----

    #[derive(Clone, Default)]
    struct SharedLog(TestLog);

    #[derive(Default, Props)]
    struct ClickAreaProps {
        log: SharedLog,
    }

    #[allow(clippy::let_underscore_future)]
    #[component]
    fn ClickArea(props: &ClickAreaProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let log = props.log.clone();
        hooks.use_click(move |c| {
            log.0
                .borrow_mut()
                .push(format!("click:{}:{:?}", c.count, c.button));
        });
        crate::element!(Text(text: "click"))
    }

    #[derive(Default, Props)]
    struct DragAreaProps {
        log: SharedLog,
    }

    #[component]
    fn DragArea(props: &DragAreaProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let log = props.log.clone();
        hooks.use_drag(move |d| {
            log.0
                .borrow_mut()
                .push(format!("{:?}:{}->{}", d.phase, d.from.x, d.to.x));
        });
        crate::element!(Text(text: "drag"))
    }

    #[derive(Default, Props)]
    struct HoverAreaProps {
        flag: Rc<Cell<bool>>,
    }

    #[component]
    fn HoverArea(props: &HoverAreaProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let hovered = hooks.use_hover();
        props.flag.set(hovered);
        crate::element!(Text(text: if hovered { "IN" } else { "out" }))
    }

    #[derive(Default, Props)]
    struct PointerAreaProps {
        log: SharedLog,
        buttons: MouseButtons,
        track_move: bool,
    }

    #[component]
    fn PointerArea(props: &PointerAreaProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let log = props.log.clone();
        let buttons = props.buttons;
        hooks.use_pointer(
            PointerOptions {
                buttons,
                layer: None,
                track_move: props.track_move,
            },
            move |ev| {
                log.0.borrow_mut().push(format!("{:?}", ev.kind));
                EventResult::Ignored
            },
        );
        crate::element!(Text(text: "ptr"))
    }

    #[derive(Default, Props)]
    struct SplitAppProps {
        log: SharedLog,
    }

    // 左右两个半屏区域:左半挂目标组件,右半纯文本(用于跨区域/离开场景)。
    #[component]
    fn SplitApp(props: &SplitAppProps, hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let log = props.log.clone();
        crate::element!(View(flex_direction: Direction::Horizontal) {
            ClickArea(log: log)
            PlainArea
        })
    }

    #[derive(Default, Props)]
    struct SplitHoverProps {
        flag: Rc<Cell<bool>>,
    }

    #[component]
    fn SplitHover(props: &SplitHoverProps, hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let flag = props.flag.clone();
        crate::element!(View(flex_direction: Direction::Horizontal) {
            HoverArea(flag: flag)
            PlainArea
        })
    }

    #[derive(Default, Props)]
    struct SplitPointerProps {
        log: SharedLog,
    }

    #[component]
    fn SplitPointer(props: &SplitPointerProps, hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let log = props.log.clone();
        crate::element!(View(flex_direction: Direction::Horizontal) {
            PointerArea(log: log, buttons: MouseButtons::LEFT, track_move: true)
            PlainArea
        })
    }

    #[component]
    fn PlainArea() -> impl Into<AnyElement<'static>> {
        crate::element!(Text(text: "plain"))
    }

    type PressLog = std::sync::Arc<std::sync::Mutex<Vec<&'static str>>>;

    #[derive(Clone, Default, Props)]
    struct ScrolledButtonsProps {
        presses: PressLog,
        nested: bool,
    }

    #[component]
    fn ScrolledButtons(
        props: &ScrolledButtonsProps,
        mut hooks: Hooks,
    ) -> impl Into<AnyElement<'static>> {
        use crate::components::{Button, ScrollView, ScrollViewState};
        use ratatui::{
            layout::{Constraint, Position},
            widgets::Block,
        };
        let outer = hooks.use_state(|| ScrollViewState::with_offset(Position::new(0, 3)));
        let inner = hooks.use_state(|| ScrollViewState::with_offset(Position::new(0, 3)));
        let first = props.presses.clone();
        let target = props.presses.clone();
        let last = props.presses.clone();
        let buttons = crate::element!(ScrollView(state: if props.nested {inner} else {outer},
            block: Block::bordered(), height: if props.nested {Constraint::Length(8)} else {Constraint::Fill(1)}) {
            Button(label: "FIRST".to_string(), on_press: move |()| first.lock().unwrap().push("first"))
            Button(label: "TARGET".to_string(), on_press: move |()| target.lock().unwrap().push("target"))
            Button(label: "THIRD".to_string(), on_press: move |()| last.lock().unwrap().push("third"))
        });
        if props.nested {
            crate::element!(ScrollView(state: outer, block: Block::bordered()) {
                View(height: Constraint::Length(3)) { Text(text: "spacer") }
                { buttons }
            })
            .into_any()
        } else {
            buttons.into_any()
        }
    }

    #[test]
    fn scrolled_button_hits_screen_position_and_clips_hidden_button() {
        for nested in [false, true] {
            let presses: PressLog = Default::default();
            let mut mounted = Mounted::mount_sized(
                crate::element!(ScrolledButtons(presses: presses.clone(), nested: nested)),
                true,
                20,
                7,
            );
            let buffer = mounted.buffer();
            assert!(crate::mouse_test::find_text(&buffer, "FIRST").is_none());
            let (x, y) = crate::mouse_test::find_text(&buffer, "TARGET").expect("target visible");
            mounted.dispatch(MouseEventKind::Down(MouseButton::Left), x, y);
            mounted.dispatch(MouseEventKind::Up(MouseButton::Left), x, y);
            assert_eq!(*presses.lock().unwrap(), ["target"], "nested={nested}");
            // 顶边框邻接被裁剪的第一个按钮,不能激活不可见条目。
            mounted.dispatch(MouseEventKind::Down(MouseButton::Left), x, 0);
            assert_eq!(*presses.lock().unwrap(), ["target"]);
        }
    }

    #[derive(Clone, Default, Props)]
    struct MovingHoverProps {
        hovered: Rc<Cell<bool>>,
        moves: TestLog,
    }

    #[component]
    fn MovingHover(props: &MovingHoverProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        use ratatui::layout::{Constraint, Offset};
        let mut shifted = hooks.use_state(|| false);
        let hovered = hooks.use_hover();
        props.hovered.set(hovered);
        let log = props.moves.clone();
        hooks.use_pointer(
            PointerOptions {
                track_move: true,
                ..Default::default()
            },
            move |event| {
                log.borrow_mut().push(format!("{:?}", event.kind));
                EventResult::Ignored
            },
        );
        hooks.use_event_handler(EventScope::Global, EventPriority::Normal, move |event| {
            if matches!(event, Event::Key(_)) {
                shifted.set(!shifted.get());
                EventResult::Consumed
            } else {
                EventResult::Ignored
            }
        });
        crate::element!(View(height:Constraint::Length(2),width:Constraint::Length(8),offset:Offset {x:0,y:if shifted.get(){4}else{0}}) { Text(text:"hover") })
    }

    #[test]
    fn stationary_pointer_leaves_and_enters_when_layout_moves() {
        let hovered = Rc::new(Cell::new(false));
        let moves = TestLog::default();
        let mut mounted = Mounted::mount_sized(
            crate::element!(MovingHover(hovered:hovered.clone(),moves:moves.clone())),
            true,
            10,
            8,
        );
        mounted.dispatch(MouseEventKind::Moved, 1, 1);
        assert!(hovered.get());
        mounted.dispatch_key(crossterm::event::KeyCode::Char('m'));
        assert!(!hovered.get());
        mounted.dispatch_key(crossterm::event::KeyCode::Char('m'));
        assert!(hovered.get());
        assert_eq!(*moves.borrow(), log_of(&["Enter", "Leave", "Enter"]));
    }

    #[test]
    fn stationary_pointer_hover_reconciles_after_terminal_resize() {
        let hovered = Rc::new(Cell::new(false));
        let moves = TestLog::default();
        let mut mounted = Mounted::mount_sized(
            crate::element!(MovingHover(hovered:hovered.clone(),moves:moves.clone())),
            true,
            10,
            8,
        );
        mounted.dispatch(MouseEventKind::Moved, 5, 1);
        assert!(hovered.get());
        mounted.resize(3, 8);
        assert!(!hovered.get());
        mounted.resize(10, 8);
        assert!(hovered.get());
        assert_eq!(*moves.borrow(), log_of(&["Enter", "Leave", "Enter"]));
    }

    type CoordinateLog = std::sync::Arc<std::sync::Mutex<Vec<(PointerKind, Position, Position)>>>;
    #[derive(Clone, Default, Props)]
    struct CoordinateProps {
        log: CoordinateLog,
    }

    #[component]
    fn CapturedCoordinates(
        props: &CoordinateProps,
        mut hooks: Hooks,
    ) -> impl Into<AnyElement<'static>> {
        use ratatui::layout::Constraint;
        let log = props.log.clone();
        hooks.use_pointer(PointerOptions::default(), move |event| {
            log.lock()
                .unwrap()
                .push((event.kind, event.position, event.screen_position));
            EventResult::Consumed
        });
        crate::element!(View(height:Constraint::Length(12),width:Constraint::Length(18)) { Text(text:"drag") })
    }

    #[component]
    fn CoordinateScroll(
        props: &CoordinateProps,
        mut hooks: Hooks,
    ) -> impl Into<AnyElement<'static>> {
        use crate::components::{ScrollView, ScrollViewState};
        let state = hooks.use_state(|| ScrollViewState::with_offset(Position::new(0, 3)));
        let log = props.log.clone();
        crate::element!(ScrollView(state:state,block:ratatui::widgets::Block::bordered()) { CapturedCoordinates(log:log) })
    }

    #[test]
    fn captured_pointer_preserves_screen_coordinates_and_maps_outside_drag() {
        let log: CoordinateLog = Default::default();
        let mut mounted = Mounted::mount_sized(
            crate::element!(CoordinateScroll(log:log.clone())),
            true,
            20,
            7,
        );
        mounted.dispatch(MouseEventKind::Down(MouseButton::Left), 4, 2);
        mounted.dispatch(MouseEventKind::Drag(MouseButton::Left), 30, 9);
        mounted.dispatch(MouseEventKind::Up(MouseButton::Left), 30, 9);
        assert_eq!(
            *log.lock().unwrap(),
            [
                (
                    PointerKind::Down(MouseButton::Left),
                    Position::new(3, 4),
                    Position::new(4, 2)
                ),
                (
                    PointerKind::Drag(MouseButton::Left),
                    Position::new(29, 11),
                    Position::new(30, 9)
                ),
                (
                    PointerKind::Up(MouseButton::Left),
                    Position::new(29, 11),
                    Position::new(30, 9)
                ),
            ]
        );
    }

    // ---- use_click ----

    // 单击 + 双击计数
    #[test]
    fn click_single_then_double() {
        let log = SharedLog(TestLog::default());
        let mut m = Mounted::mount(crate::element!(ClickArea(log: log.clone())), true);
        m.dispatch(MouseEventKind::Down(MouseButton::Left), 2, 1);
        m.dispatch(MouseEventKind::Up(MouseButton::Left), 2, 1);
        m.dispatch(MouseEventKind::Down(MouseButton::Left), 2, 1);
        m.dispatch(MouseEventKind::Up(MouseButton::Left), 2, 1);
        assert_eq!(*log.0.borrow(), log_of(&["click:1:Left", "click:2:Left"]));
    }

    // 跨目标:左半屏按下,释放落在右半屏 → 不合成点击
    #[test]
    fn click_cross_target_not_synthesized() {
        let log = SharedLog(TestLog::default());
        let mut m = Mounted::mount(crate::element!(SplitApp(log: log.clone())), true);
        m.dispatch(MouseEventKind::Down(MouseButton::Left), 2, 1);
        m.dispatch(MouseEventKind::Up(MouseButton::Left), 30, 1);
        assert!(log.0.borrow().is_empty());
    }

    // 点击默认消费:不冒泡(全屏父 handler 收不到)
    #[test]
    fn click_consumes_and_does_not_bubble() {
        let log = SharedLog(TestLog::default());
        let mut m = Mounted::mount(crate::element!(ClickArea(log: log.clone())), true);
        // 单组件场景下无父 handler;冒泡截断已由 input::tests::scrim 覆盖,此处验证消费链路完整。
        m.dispatch(MouseEventKind::Down(MouseButton::Left), 2, 1);
        m.dispatch(MouseEventKind::Up(MouseButton::Left), 2, 1);
        assert_eq!(*log.0.borrow(), log_of(&["click:1:Left"]));
    }

    // ---- use_drag ----

    #[derive(Default, Props)]
    struct SplitDragProps {
        log: SharedLog,
    }

    // 左半屏拖拽区:拖到右半屏(区域外)的 Move/End 仍应送达(指针捕获)。
    #[component]
    fn SplitDrag(props: &SplitDragProps, hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let log = props.log.clone();
        crate::element!(View(flex_direction: Direction::Horizontal) {
            DragArea(log: log)
            PlainArea
        })
    }

    // 三态:Start → Move(拖出区域仍收到,经由捕获) → End
    #[test]
    fn drag_three_phases() {
        let log = SharedLog(TestLog::default());
        let mut m = Mounted::mount(crate::element!(SplitDrag(log: log.clone())), true);
        m.dispatch(MouseEventKind::Down(MouseButton::Left), 1, 1);
        m.dispatch(MouseEventKind::Drag(MouseButton::Left), 5, 5);
        m.dispatch(MouseEventKind::Drag(MouseButton::Left), 30, 5); // 已出左半屏
        m.dispatch(MouseEventKind::Up(MouseButton::Left), 39, 9); // 区域外释放
        assert_eq!(
            *log.0.borrow(),
            log_of(&["Start:1->1", "Move:1->5", "Move:5->30", "End:30->39"])
        );
    }

    #[derive(Default, Props)]
    struct CancellableProps {
        log: SharedLog,
        blocked: Rc<Cell<bool>>,
        drag: bool,
    }

    #[component]
    fn Cancellable(props: &CancellableProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        use crate::UseInputLayer;
        let log = props.log.clone();
        if props.drag {
            hooks.use_drag(move |event| log.0.borrow_mut().push(format!("{:?}", event.phase)));
        } else {
            hooks.use_pointer(
                PointerOptions {
                    track_move: true,
                    ..Default::default()
                },
                move |event| {
                    log.0.borrow_mut().push(format!("{:?}", event.kind));
                    EventResult::Consumed
                },
            );
        }
        hooks.use_input_layer(props.blocked.get(), true);
        let blocked = props.blocked.clone();
        hooks.use_event_handler(EventScope::Global, EventPriority::Normal, move |event| {
            if matches!(event, Event::Key(_)) {
                blocked.set(!blocked.get());
                EventResult::Consumed
            } else {
                EventResult::Ignored
            }
        });
        crate::element!(Text(text: "cancellable"))
    }

    #[test]
    fn modal_cancels_pointer_without_click_or_reviving_capture() {
        let log = SharedLog(TestLog::default());
        let mut mounted = Mounted::mount(crate::element!(Cancellable(log: log.clone())), true);
        mounted.dispatch(MouseEventKind::Down(MouseButton::Left), 2, 1);
        mounted.dispatch_key(crossterm::event::KeyCode::Char('m'));
        assert_eq!(
            *log.0.borrow(),
            log_of(&["Down(Left)", "Enter", "Cancel(Left)"])
        );
        mounted.dispatch_key(crossterm::event::KeyCode::Char('m'));
        mounted.dispatch(MouseEventKind::Up(MouseButton::Left), 2, 1);
        assert_eq!(
            *log.0.borrow(),
            log_of(&["Down(Left)", "Enter", "Cancel(Left)", "Enter", "Up(Left)"])
        );
    }

    #[test]
    fn modal_cancels_drag_and_clears_local_active_state() {
        let log = SharedLog(TestLog::default());
        let mut mounted = Mounted::mount(
            crate::element!(Cancellable(log: log.clone(), drag: true)),
            true,
        );
        mounted.dispatch(MouseEventKind::Down(MouseButton::Left), 2, 1);
        mounted.dispatch_key(crossterm::event::KeyCode::Char('m'));
        mounted.dispatch_key(crossterm::event::KeyCode::Char('m'));
        mounted.dispatch(MouseEventKind::Drag(MouseButton::Left), 2, 2);
        mounted.dispatch(MouseEventKind::Up(MouseButton::Left), 2, 2);
        assert_eq!(*log.0.borrow(), log_of(&["Start", "Cancel"]));
    }

    #[test]
    fn modal_clears_pointer_hover_without_another_move() {
        let log = SharedLog(TestLog::default());
        let mut mounted = Mounted::mount(crate::element!(Cancellable(log: log.clone())), true);
        mounted.dispatch(MouseEventKind::Moved, 2, 1);
        mounted.dispatch_key(crossterm::event::KeyCode::Char('m'));
        assert_eq!(*log.0.borrow(), log_of(&["Enter", "Leave"]));
        mounted.dispatch_key(crossterm::event::KeyCode::Char('m'));
        mounted.dispatch(MouseEventKind::Moved, 2, 1);
        assert_eq!(
            *log.0.borrow(),
            log_of(&["Enter", "Leave", "Enter", "Move"])
        );
    }

    // ---- use_hover ----

    // 移入/移出反映状态
    #[test]
    fn hover_enter_leave_state() {
        let flag = Rc::new(Cell::new(false));
        let mut m = Mounted::mount(crate::element!(SplitHover(flag: flag.clone())), true);
        assert!(!flag.get());
        m.dispatch(MouseEventKind::Moved, 2, 2); // 左半
        assert!(flag.get());
        m.dispatch(MouseEventKind::Moved, 39, 9); // 右半 → 离开
        assert!(!flag.get());
    }

    // ---- use_pointer ----

    // kind 映射:Down/Up/Click/Scroll
    #[test]
    fn pointer_kinds_mapping() {
        let log = SharedLog(TestLog::default());
        let mut m = Mounted::mount(
            crate::element!(PointerArea(
                log: log.clone(),
                buttons: MouseButtons::ALL,
                track_move: false,
            )),
            true,
        );
        m.dispatch(MouseEventKind::Down(MouseButton::Left), 2, 1);
        m.dispatch(MouseEventKind::Up(MouseButton::Left), 2, 1);
        m.dispatch(MouseEventKind::ScrollDown, 2, 1);
        assert_eq!(
            *log.0.borrow(),
            log_of(&[
                "Down(Left)",
                "Up(Left)",
                "Click { button: Left, count: 1 }",
                "ScrollDown"
            ])
        );
    }

    // 按键过滤:默认 LEFT,右键 Down 不投递
    #[test]
    fn pointer_default_options_receive_left_click_only() {
        let log = SharedLog(TestLog::default());
        let mut m = Mounted::mount(crate::element!(PointerArea(log: log.clone())), true);
        m.dispatch(MouseEventKind::Down(MouseButton::Right), 2, 1);
        m.dispatch(MouseEventKind::Up(MouseButton::Right), 2, 1);
        m.dispatch(MouseEventKind::Down(MouseButton::Left), 2, 1);
        m.dispatch(MouseEventKind::Up(MouseButton::Left), 2, 1);
        assert_eq!(
            *log.0.borrow(),
            log_of(&["Down(Left)", "Up(Left)", "Click { button: Left, count: 1 }"])
        );
    }

    #[test]
    fn pointer_buttons_filter() {
        let log = SharedLog(TestLog::default());
        let mut m = Mounted::mount(
            crate::element!(PointerArea(
                log: log.clone(),
                buttons: MouseButtons::RIGHT,
                track_move: false,
            )),
            true,
        );
        m.dispatch(MouseEventKind::Down(MouseButton::Left), 2, 1);
        m.dispatch(MouseEventKind::Up(MouseButton::Left), 2, 1);
        assert!(log.0.borrow().is_empty());
        m.dispatch(MouseEventKind::Down(MouseButton::Right), 2, 1);
        m.dispatch(MouseEventKind::Up(MouseButton::Right), 2, 1);
        assert_eq!(
            *log.0.borrow(),
            log_of(&[
                "Down(Right)",
                "Up(Right)",
                "Click { button: Right, count: 1 }"
            ])
        );
    }

    // track_move:Enter → Move → Leave,区域外移动不投递
    #[test]
    fn pointer_move_enter_leave() {
        let log = SharedLog(TestLog::default());
        let mut m = Mounted::mount(crate::element!(SplitPointer(log: log.clone())), true);
        m.dispatch(MouseEventKind::Moved, 2, 2);
        m.dispatch(MouseEventKind::Moved, 3, 3);
        m.dispatch(MouseEventKind::Moved, 39, 9);
        m.dispatch(MouseEventKind::Moved, 38, 9);
        assert_eq!(*log.0.borrow(), log_of(&["Enter", "Move", "Leave"]));
    }

    // ---- 逃生门冒泡(规格:原始指针流) ----

    // 内层 use_pointer 放行(Ignored)→ 事件继续冒泡,外层组件的 handler 收到。
    #[derive(Default, Props)]
    struct BubbleAppProps {
        log: SharedLog,
    }

    #[component]
    fn BubbleApp(props: &BubbleAppProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let outer_log = props.log.clone();
        hooks.use_event_handler_with_options(
            EventScope::Current,
            EventPriority::Normal,
            EventOptions { hit_test: false },
            move |event| {
                if let Event::Mouse(m) = event {
                    outer_log.0.borrow_mut().push(format!("outer:{:?}", m.kind));
                    return EventResult::Consumed;
                }
                EventResult::Ignored
            },
        );
        // 内层 PointerArea 对所有事件返回 Ignored(见其定义)。
        crate::element!(PointerArea(
            log: props.log.clone(),
            buttons: MouseButtons::ALL,
            track_move: false,
        ))
    }

    #[test]
    fn pointer_ignored_bubbles_to_outer_handler() {
        let log = SharedLog(TestLog::default());
        let mut m = Mounted::mount(crate::element!(BubbleApp(log: log.clone())), true);
        m.dispatch(MouseEventKind::ScrollDown, 2, 1);
        // 内层先收到(绘制序倒排,子先于父),Ignored 后外层收到。
        assert_eq!(*log.0.borrow(), log_of(&["ScrollDown", "outer:ScrollDown"]));
    }

    // Button 消费 Down(外层不收到);Button 放行 Scroll(外层收到)。
    #[test]
    fn button_press_consumes_but_scroll_bubbles() {
        use crate::components::Button;
        use std::sync::{Arc, Mutex};

        type SyncLog = Arc<Mutex<Vec<String>>>;
        #[derive(Clone, Default, Props)]
        struct OuterButtonProps {
            raw: SyncLog,
        }

        #[component]
        fn OuterButton(
            props: &OuterButtonProps,
            mut hooks: Hooks,
        ) -> impl Into<AnyElement<'static>> {
            let raw = props.raw.clone();
            hooks.use_event_handler_with_options(
                EventScope::Current,
                EventPriority::Normal,
                EventOptions { hit_test: false },
                move |event| {
                    if let Event::Mouse(m) = event {
                        raw.lock().unwrap().push(format!("{:?}", m.kind));
                        return EventResult::Consumed;
                    }
                    EventResult::Ignored
                },
            );
            crate::element!(Button(label: "OK".to_string(), on_press: |()| {}))
        }

        let raw: SyncLog = Default::default();
        let mut m = Mounted::mount(crate::element!(OuterButton(raw: raw.clone())), true);
        // Down 命中按钮:Button 消费,外层不收到。
        m.dispatch(MouseEventKind::Down(MouseButton::Left), 2, 1);
        assert!(raw.lock().unwrap().is_empty());
        // Scroll:Button 放行 → 冒泡到外层。
        m.dispatch(MouseEventKind::ScrollDown, 2, 1);
        assert_eq!(*raw.lock().unwrap(), vec!["ScrollDown".to_string()]);
    }

    // ---- 1.3 哑坑断言 ----

    // 未启用鼠标捕获而注册指针 hook → debug 断言并给出指引
    #[test]
    #[should_panic(expected = "mouse capture is not enabled")]
    fn pointer_hook_without_mouse_panics_in_debug() {
        let log = SharedLog(TestLog::default());
        let _ = Mounted::mount(crate::element!(ClickArea(log: log)), false);
    }
}
