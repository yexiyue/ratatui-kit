// ScrollView 组件：可滚动视图容器，支持横向/纵向滚动条，适合长列表、文档阅读等场景。
//
// ## 用法示例
//
// ### 自动管理滚动状态（推荐）
// ```rust
// element!(ScrollView {
//     // 子内容(内置键鼠滚动由 active 默认开启)
// })
// ```
//
// ### 外部状态(读偏移 / 程序化滚动;与 active 正交,不会关掉内置滚动)
// ```rust
// let scroll_state = hooks.use_state(ScrollViewState::default);
//
// element!(ScrollView(
//     state: scroll_state,
//     scrollbars: Scrollbars::default(),
// ){
//     // 子内容
// })
// ```
//
// ScrollView 的两种模式是正交的：
// 1. 不传 `state`，组件用内部滚动状态；
// 2. 传 `state`，页面可读偏移、程序化滚动(`scroll_to_visible` / `is_at_bottom`)，
//    同时 `active`(默认 true)仍提供内置键鼠滚动。
//
// `Scrollbars::over_border`(默认 true)控制滚动条盖在 block 边框上还是退到框内。

use crate::{AnyElement, Component, layout_style::LayoutStyle};
use crate::{
    Hook, PointerOptions, State, UseEventHandler, UsePointer, UseState,
    input::{EventOptions, EventPriority, EventResult, EventScope},
};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect, Size},
    widgets::{Block, Borders},
};
use ratatui_kit_macros::{Props, with_layout_style};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
mod state;
pub use state::ScrollViewState;
mod scrollbars;
use scrollbars::ResolvedLayout;
pub(crate) use scrollbars::{AxisGeometry, ScrollbarGeometry, ScrollbarInteraction};
pub use scrollbars::{ScrollbarStyle, ScrollbarVisibility, Scrollbars};

#[with_layout_style]
#[derive(Props)]
// ScrollView 组件属性。
pub struct ScrollViewProps<'a> {
    // 子元素列表。
    pub children: Vec<AnyElement<'a>>,
    // 滚动条配置。
    pub scrollbars: Scrollbars<'static>,
    // 外部滚动状态(与 `active` 正交:传了也不会关掉内置滚动)。
    pub state: Option<State<ScrollViewState>>,

    // 可选边框块。
    pub block: Option<Block<'static>>,

    // 是否启用内置键鼠滚动(默认 true),与其它选择类组件的 `active` 约定一致。
    pub active: bool,
}

impl Default for ScrollViewProps<'_> {
    fn default() -> Self {
        Self {
            children: Vec::new(),
            scrollbars: Scrollbars::default(),
            state: None,
            block: None,
            active: true,
            margin: Default::default(),
            offset: Default::default(),
            width: Default::default(),
            height: Default::default(),
            gap: Default::default(),
            flex_direction: Default::default(),
            justify_content: Default::default(),
        }
    }
}

// ScrollView 组件实现。
pub struct ScrollView {
    scrollbars: Scrollbars<'static>,
    block: Option<Block<'static>>,
    border_ring: bool,
    // draw() 在把 area 缩成 block.inner 之前暂存外框,只用于本帧布局解析。
    outer: Option<Rect>,
    // 解析后的滚动状态(内部或外部),供 calc_children_areas 记录子节点区域以支持 scroll_to_index。
    scroll_view_state: Option<State<ScrollViewState>>,
    resolved: Rc<Cell<Option<ResolvedLayout>>>,
}

// ratatui Block 没有 borders getter。其 Eq 能精确验证边框组合,避免把 padding/title
// 误认为边框;只在 block props 改变时探测,不会在布局/绘制阶段重复克隆。
fn has_border_ring(block: &Block<'_>) -> bool {
    [
        Borders::ALL,
        Borders::RIGHT | Borders::BOTTOM,
        Borders::LEFT | Borders::RIGHT | Borders::BOTTOM,
        Borders::TOP | Borders::RIGHT | Borders::BOTTOM,
    ]
    .into_iter()
    .any(|borders| *block == block.clone().borders(borders))
}

fn clamp_u16(value: u128) -> u16 {
    value.min(u16::MAX as u128) as u16
}

fn constraints_to_lengths(constraints: impl Iterator<Item = Constraint>, len: u16) -> Vec<u16> {
    constraints
        .map(|constraint| match constraint {
            Constraint::Length(value) => value,
            Constraint::Percentage(percent) => {
                clamp_u16(u128::from(len) * u128::from(percent) / 100)
            }
            Constraint::Ratio(numerator, denominator) => {
                if denominator == 0 {
                    0
                } else {
                    clamp_u16(u128::from(len) * u128::from(numerator) / u128::from(denominator))
                }
            }
            Constraint::Min(value) => value,
            Constraint::Max(value) => value,
            Constraint::Fill(weight) => clamp_u16(u128::from(len) * u128::from(weight)),
        })
        .collect()
}

fn gap_sum(count: usize, gap: i32) -> u16 {
    if count == 0 {
        return 0;
    }

    let total = count.saturating_sub(1) as i128 * i128::from(gap);
    if total <= 0 {
        0
    } else {
        clamp_u16(total as u128)
    }
}

fn sum_with_gap(lengths: &[u16], gap: i32) -> u16 {
    if lengths.is_empty() {
        return 0;
    }

    let sum = lengths
        .iter()
        .fold(0u128, |sum, value| sum.saturating_add(u128::from(*value)));
    clamp_u16(sum.saturating_add(u128::from(gap_sum(lengths.len(), gap))))
}

fn cross_direction(direction: Direction) -> Direction {
    match direction {
        Direction::Horizontal => Direction::Vertical,
        Direction::Vertical => Direction::Horizontal,
    }
}

fn area_len(area: Rect, direction: Direction) -> u16 {
    match direction {
        Direction::Horizontal => area.width,
        Direction::Vertical => area.height,
    }
}

fn lengths_to_constraints(lengths: &[u16]) -> Vec<Constraint> {
    lengths
        .iter()
        .map(|length| Constraint::Length(*length))
        .collect()
}

fn content_size(
    direction: Direction,
    main_lengths: &[u16],
    cross_lengths: &[u16],
    gap: i32,
) -> (u16, u16) {
    let main = sum_with_gap(main_lengths, gap);
    let cross = cross_lengths.iter().max().copied().unwrap_or_default();

    match direction {
        Direction::Horizontal => (main, cross),
        Direction::Vertical => (cross, main),
    }
}

impl Component for ScrollView {
    type Props<'a> = ScrollViewProps<'a>;

    fn new(props: &Self::Props<'_>) -> Self {
        Self {
            scrollbars: props.scrollbars.clone(),
            block: props.block.clone(),
            border_ring: props.block.as_ref().is_some_and(has_border_ring),
            outer: None,
            scroll_view_state: None,
            resolved: Rc::default(),
        }
    }

    fn update(
        &mut self,
        props: &mut Self::Props<'_>,
        mut hooks: crate::Hooks,
        updater: &mut crate::ComponentUpdater,
    ) {
        // 手写 Component 的 hooks 默认 context=None;先升级为 context-aware 以便用 use_event_handler。
        // 所有 hooks 操作须置于后续 `&mut updater`(set_layout_style / update_children)之前。
        let mut hooks = hooks.with_context_stack(updater.component_context_stack());

        let layout_style = props.layout_style();

        let this_scroll_view_state = hooks.use_state(ScrollViewState::default);
        // 外部 state 与 active 正交:传外部 state 也不关掉内置滚动(与 Select/Table 一致)。
        let state = props.state.unwrap_or(this_scroll_view_state);
        let active = props.active;
        if self.block != props.block {
            self.block = props.block.clone();
            self.border_ring = self.block.as_ref().is_some_and(has_border_ring);
        }
        let pointer_shared = {
            let hook = hooks.use_hook(|| UseScrollImpl {
                scrollbars: props.scrollbars.clone(),
                resolved: Rc::clone(&self.resolved),
                pointer: Rc::default(),
            });
            hook.scrollbars = props.scrollbars.clone();

            Rc::clone(&hook.pointer)
        };

        // 滚动事件:Current 层 + 鼠标命中过滤。命中的滚动键/滚轮返回 Consumed,不再无声漏给兄弟 handler。
        hooks.use_event_handler_with_options(
            EventScope::Current,
            EventPriority::Normal,
            EventOptions { hit_test: true },
            move |event| {
                if active && state.write().handle_event(&event) {
                    EventResult::Consumed
                } else {
                    EventResult::Ignored
                }
            },
        );

        // 滚动条拖拽:抓住滑块拖动 / 点滑轨空白让滑块跳到指针处。仅应用启用鼠标捕获时
        // 注册(`use_pointer` 未启用时 debug 断言;`mouse_enabled` 进程内不变,条件分支
        // 跨帧稳定,不违反 hook 顺序)。不受 `active` 门控 —— 抓住滑块就是要滚这个视图,
        // 与键/轮的「活跃」语义无关。
        if crate::hooks::use_pointer::pointer_enabled(&hooks) {
            hooks.use_pointer(
                PointerOptions {
                    track_move: true,
                    ..Default::default()
                },
                move |event| {
                    pointer_shared
                        .borrow_mut()
                        .handle(event, |target, vertical| {
                            let mut view = state.write();
                            let mut offset = view.offset;
                            if vertical {
                                offset.y = target as u16;
                            } else {
                                offset.x = target as u16;
                            }
                            view.set_offset(offset);
                        })
                },
            );
        }

        self.scrollbars = props.scrollbars.clone();
        self.scroll_view_state = Some(state);

        updater.set_layout_style(layout_style);
        updater.update_children(&mut props.children, None);
    }

    fn calc_children_areas(
        &self,
        children: &crate::Components,
        layout_style: &LayoutStyle,
        drawer: &mut crate::ComponentDrawer<'_, '_>,
    ) -> Vec<ratatui::prelude::Rect> {
        let inner = drawer.area;
        let main_direction = layout_style.flex_direction;
        let cross_axis = cross_direction(main_direction);
        // 主轴的百分比/Fill以原inner为参照,不能因横/纵轨道扣减再次缩短内容。
        let main_lengths = constraints_to_lengths(
            children.constraints(main_direction),
            area_len(inner, main_direction),
        );
        let mut cross_lengths = Vec::new();
        let mut resolved = self.scrollbars.resolve_layout(
            self.outer.unwrap_or(inner),
            inner,
            self.border_ring,
            |viewport| {
                cross_lengths = constraints_to_lengths(
                    children.constraints(cross_axis),
                    match cross_axis {
                        Direction::Horizontal => viewport.width,
                        Direction::Vertical => viewport.height,
                    },
                );
                let (width, height) = content_size(
                    main_direction,
                    &main_lengths,
                    &cross_lengths,
                    layout_style.gap,
                );
                Size::new(width, height)
            },
        );
        let offset = if let Some(state) = self.scroll_view_state {
            let mut state = state.write_no_update();
            state.size = Some(resolved.content);
            state.page_size = Some(resolved.viewport.as_size());
            state.offset.x = state.offset.x.min(
                resolved
                    .content
                    .width
                    .saturating_sub(resolved.viewport.width),
            );
            state.offset.y = state.offset.y.min(
                resolved
                    .content
                    .height
                    .saturating_sub(resolved.viewport.height),
            );
            state.offset
        } else {
            Default::default()
        };
        resolved.offset = offset;
        self.resolved.set(Some(resolved));
        let justify_constraints = lengths_to_constraints(&main_lengths);
        let align_constraints = lengths_to_constraints(&cross_lengths);

        let rect = Rect::new(0, 0, resolved.content.width, resolved.content.height);
        drawer.push_scroll_buffer(Buffer::empty(rect), resolved.viewport, offset);

        drawer.area = drawer.buffer_mut().area;

        // flex layout
        let layout = layout_style.get_layout().constraints(justify_constraints);
        let areas = layout.split(drawer.area);

        let mut new_areas = Vec::with_capacity(areas.len());

        let rev_direction = cross_direction(layout_style.flex_direction);
        for (area, constraint) in areas.iter().zip(align_constraints.iter()) {
            let area = Layout::new(rev_direction, [constraint]).split(*area)[0];
            new_areas.push(area);
        }

        // 记录子节点在内容缓冲中的区域,供 `ScrollViewState::scroll_to_index` 联动滚动。
        // 用 write_no_update:布局阶段写入不应触发重渲染(与 hook 写 offset 同理)。
        if let Some(state) = self.scroll_view_state {
            state.write_no_update().child_areas = new_areas.clone();
        }

        new_areas
    }

    fn draw(&mut self, drawer: &mut crate::ComponentDrawer<'_, '_>) {
        // Block inner 只在这里计算一次;后续布局与绘制共享 ResolvedLayout。
        self.outer = Some(drawer.area);
        if let Some(block) = self.block.as_ref() {
            let inner_area = block.inner(drawer.area);
            drawer.render_widget(block, drawer.area);
            drawer.area = inner_area;
        }
    }
}

struct UseScrollImpl {
    scrollbars: Scrollbars<'static>,
    resolved: Rc<Cell<Option<ResolvedLayout>>>,
    // 指针拖拽共享态:post_component_draw 回填本帧几何,update 里注册的指针回调读取/驱动。
    pointer: Rc<RefCell<ScrollbarInteraction>>,
}

impl Hook for UseScrollImpl {
    fn post_component_draw(&mut self, drawer: &mut crate::ComponentDrawer) {
        // pop 本层内容缓冲(嵌套安全:guard 避免 unwrap on None);pop 后 buffer_mut 回到外层。
        let Some(buffer) = drawer.pop_scroll_buffer() else {
            return;
        };
        let Some(resolved) = self.resolved.get() else {
            return;
        };
        let geometry = self.scrollbars.render_ref(
            resolved,
            drawer.buffer_mut(),
            &buffer,
            &self.pointer.borrow(),
        );
        self.pointer.borrow_mut().geometry = geometry;
    }
}

#[cfg(test)]
mod drag_tests {
    use super::*;
    use crate::{
        Hooks,
        components::{Text, View},
        mouse_test::Mounted,
    };
    use crossterm::event::{MouseButton, MouseEventKind};
    use ratatui::layout::Direction;
    use ratatui_kit_macros::{Props, component};
    use std::{cell::RefCell, rc::Rc};

    type StateSlot = Rc<RefCell<Option<State<ScrollViewState>>>>;

    #[derive(Clone, Default, Props)]
    struct HarnessProps {
        slot: StateSlot,
    }

    // 30 行内容 + 40x10 终端 → 纵向滚动条出现在最后一列(x=39)。
    #[component]
    fn VerticalHarness(props: &HarnessProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let state = hooks.use_state(ScrollViewState::default);
        if props.slot.borrow_mut().is_none() {
            *props.slot.borrow_mut() = Some(state);
        }
        crate::element!(ScrollView(state: state) {
            for i in 0..30u16 {
                View(key: i, height: Constraint::Length(1)) { Text(text: format!("row {i}")) }
            }
        })
    }

    // 3 个宽 30 的子节点 + 横向布局 → 横向滚动条出现在最后一行(y=9)。
    #[component]
    fn HorizontalHarness(props: &HarnessProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let state = hooks.use_state(ScrollViewState::default);
        if props.slot.borrow_mut().is_none() {
            *props.slot.borrow_mut() = Some(state);
        }
        crate::element!(ScrollView(state: state, flex_direction: Direction::Horizontal) {
            for i in 0..3u16 {
                View(key: i, width: Constraint::Length(30), height: Constraint::Length(1)) {
                    Text(text: format!("col {i}"))
                }
            }
        })
    }

    #[component]
    fn PercentageHarness(props: &HarnessProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let state = hooks.use_state(ScrollViewState::default);
        *props.slot.borrow_mut() = Some(state);
        crate::element!(ScrollView(state: state) {
            View(width: Constraint::Fill(1), height: Constraint::Percentage(100)) {
                Text(text: "first")
            }
            View(width: Constraint::Fill(1), height: Constraint::Length(1)) {
                Text(text: "last")
            }
        })
    }

    #[test]
    fn percentage_content_and_scrollbar_share_one_resolution() {
        use crossterm::event::KeyCode;
        let slot: StateSlot = Default::default();
        let mut mounted = Mounted::mount_sized(
            crate::element!(PercentageHarness(slot: slot.clone())),
            true,
            10,
            5,
        );
        let state = slot.borrow().unwrap();
        assert_eq!(state.read().size(), Some(Size::new(9, 6)));
        assert_eq!(state.read().page_size(), Some(Size::new(9, 5)));
        assert_eq!(state.read().child_area(1), Some(Rect::new(0, 5, 9, 1)));
        state.write().scroll_to_index(1);
        mounted.dispatch_key(KeyCode::End);
        assert_eq!(state.read().offset().y, 1);
        assert!(state.read().is_at_bottom());
        assert_eq!(
            crate::mouse_test::find_text(&mounted.buffer(), "last"),
            Some((0, 4))
        );
    }

    fn mount_vertical(slot: &StateSlot) -> Mounted {
        let m = Mounted::mount(crate::element!(VerticalHarness(slot: slot.clone())), true);
        // 首帧后 state 句柄已存入 slot。
        assert!(slot.borrow().is_some(), "外部 state 应已捕获");
        m
    }

    fn offset_y(slot: &StateSlot) -> u16 {
        slot.borrow().unwrap().read().offset().y
    }

    fn offset_x(slot: &StateSlot) -> u16 {
        slot.borrow().unwrap().read().offset().x
    }

    // 抓住滑块拖动:offset=0 时滑块中心在 track 第 1.5 格(usable=8、thumb=3、max=20)。
    // 从 (39,2) 抓住拖到 (39,5) → 中心 4.5 → offset 11。
    #[test]
    fn dragging_thumb_scrolls_proportionally() {
        let slot: StateSlot = Default::default();
        let mut m = mount_vertical(&slot);

        m.dispatch(MouseEventKind::Down(MouseButton::Left), 39, 2);
        assert_eq!(offset_y(&slot), 0, "按在滑块上不产生跳动");
        m.dispatch(MouseEventKind::Drag(MouseButton::Left), 39, 5);
        assert_eq!(offset_y(&slot), 11);
    }

    #[test]
    fn scrollbar_feedback_clears_on_release() {
        use ratatui::style::Modifier;
        let slot: StateSlot = Default::default();
        let mut m = mount_vertical(&slot);
        m.dispatch(MouseEventKind::Moved, 39, 2);
        assert!(m.buffer()[(39, 2)].modifier.contains(Modifier::BOLD));
        m.dispatch(MouseEventKind::Down(MouseButton::Left), 39, 2);
        assert!(m.buffer()[(39, 2)].modifier.contains(Modifier::REVERSED));
        m.dispatch(MouseEventKind::Up(MouseButton::Left), 39, 2);
        assert!(!m.buffer()[(39, 2)].modifier.contains(Modifier::REVERSED));
    }

    // 拖到滑轨两端 → 偏移精确到达 0 / max(=20),端点无死区。
    #[test]
    fn drag_reaches_both_ends_exactly() {
        let slot: StateSlot = Default::default();
        let mut m = mount_vertical(&slot);

        m.dispatch(MouseEventKind::Down(MouseButton::Left), 39, 2);
        m.dispatch(MouseEventKind::Drag(MouseButton::Left), 39, 9);
        assert_eq!(offset_y(&slot), 20, "拖到底 = max_position");
        m.dispatch(MouseEventKind::Up(MouseButton::Left), 39, 9);

        m.dispatch(MouseEventKind::Down(MouseButton::Left), 39, 2);
        m.dispatch(MouseEventKind::Drag(MouseButton::Left), 39, 0);
        assert_eq!(offset_y(&slot), 0, "拖到顶 = 0(经过再渲染后回程也成立)");
    }

    // 点滑轨空白:滑块中心直接对齐指针(跳转),从指针处开始拖。
    #[test]
    fn clicking_track_jumps_thumb_to_pointer() {
        let slot: StateSlot = Default::default();
        let mut m = mount_vertical(&slot);

        m.dispatch(MouseEventKind::Down(MouseButton::Left), 39, 7);
        assert_eq!(offset_y(&slot), 17, "中心 6 → round((6-1.5)*30/8) = 17");
    }

    // 点内容区(非滑轨)不启动拖拽;后续 Drag 也不生效。
    #[test]
    fn clicking_content_does_not_start_drag() {
        let slot: StateSlot = Default::default();
        let mut m = mount_vertical(&slot);

        m.dispatch(MouseEventKind::Down(MouseButton::Left), 10, 5);
        assert_eq!(offset_y(&slot), 0);
        m.dispatch(MouseEventKind::Drag(MouseButton::Left), 39, 5);
        assert_eq!(offset_y(&slot), 0, "未抓住滑块的 Drag 不滚动");
    }

    // 指针捕获:拖动途中指针滑出滑轨列(进入内容区),沿主轴仍持续换算。
    #[test]
    fn drag_survives_leaving_track_column() {
        let slot: StateSlot = Default::default();
        let mut m = mount_vertical(&slot);

        m.dispatch(MouseEventKind::Down(MouseButton::Left), 39, 2);
        m.dispatch(MouseEventKind::Drag(MouseButton::Left), 30, 6);
        assert_eq!(
            offset_y(&slot),
            15,
            "只用主轴坐标:y=6 → 中心 5.5 → round(4*30/8) = 15"
        );
    }

    // 横向轴对称:90 列内容、viewport 40 → max=50;从 (10,9) 抓住拖到 (30,9) → 47。
    #[test]
    fn dragging_horizontal_thumb_scrolls() {
        let slot: StateSlot = Default::default();
        let mut m = Mounted::mount(crate::element!(HorizontalHarness(slot: slot.clone())), true);

        m.dispatch(MouseEventKind::Down(MouseButton::Left), 10, 9);
        m.dispatch(MouseEventKind::Drag(MouseButton::Left), 30, 9);
        assert_eq!(offset_x(&slot), 47);
    }
}
