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
    Hook, State, UseEventHandler, UseState,
    input::{EventOptions, EventPriority, EventResult, EventScope},
};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect, Size},
    widgets::Block,
};
use ratatui_kit_macros::{Props, with_layout_style};
mod state;
pub use state::ScrollViewState;
mod scrollbars;
pub use scrollbars::{ScrollbarVisibility, Scrollbars};

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
    // draw() 在把 area 缩成 block.inner 之前暂存的外框;供 calc_children_areas 与 render_ref
    // 共用同一个 `ring` 几何判定(单一真源)。
    outer: Option<Rect>,
}

fn clamp_u16(value: u128) -> u16 {
    value.min(u16::MAX as u128) as u16
}

fn constraints_to_lengths(constraints: &[Constraint], len: u16) -> Vec<u16> {
    constraints
        .iter()
        .map(|constraint| match constraint {
            Constraint::Length(value) => *value,
            Constraint::Percentage(percent) => {
                clamp_u16(u128::from(len) * u128::from(*percent) / 100)
            }
            Constraint::Ratio(numerator, denominator) => {
                if *denominator == 0 {
                    0
                } else {
                    clamp_u16(u128::from(len) * u128::from(*numerator) / u128::from(*denominator))
                }
            }
            Constraint::Min(value) => *value,
            Constraint::Max(value) => *value,
            Constraint::Fill(weight) => clamp_u16(u128::from(len) * u128::from(*weight)),
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
            outer: None,
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
        self.block = props.block.clone();

        {
            let hook = hooks.use_hook(|| UseScrollImpl {
                scroll_view_state: state,
                scrollbars: props.scrollbars.clone(),
                outer: None,
                block: props.block.clone(),
            });
            hook.scroll_view_state = state;
            hook.scrollbars = props.scrollbars.clone();
            hook.block = props.block.clone();
        }

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

        self.scrollbars = props.scrollbars.clone();

        updater.set_layout_style(layout_style);
        updater.update_children(&mut props.children, None);
    }

    fn calc_children_areas(
        &self,
        children: &crate::Components,
        layout_style: &LayoutStyle,
        drawer: &mut crate::ComponentDrawer<'_, '_>,
    ) -> Vec<ratatui::prelude::Rect> {
        let constraint_sum =
            |d: Direction, len: u16| constraints_to_lengths(&children.get_constraints(d), len);

        let axis_lengths = |area: Rect| {
            let main_direction = layout_style.flex_direction;
            let cross_direction = cross_direction(main_direction);
            let main_lengths = constraint_sum(main_direction, area_len(area, main_direction));
            let cross_lengths = constraint_sum(cross_direction, area_len(area, cross_direction));
            (main_lengths, cross_lengths)
        };

        // 此处 `drawer.area` 已是 `block.inner()`(draw() 在 calc 之前设置)。先按 inner 算一遍子长度。
        let inner = drawer.area;
        let (mut main_lengths, mut cross_lengths) = axis_lengths(inner);
        let old_width_height = content_size(
            layout_style.flex_direction,
            &main_lengths,
            &cross_lengths,
            layout_style.gap,
        );

        // ring(盖边框)与 render_ref 共用同一几何判定(单一真源);ring 时子节点铺满整个 inner,否则扣掉滚动条。
        let ring = self.scrollbars.ring(self.outer.unwrap_or(inner), inner);
        let content_area = self.scrollbars.content_area(
            inner,
            Size::new(old_width_height.0, old_width_height.1),
            ring,
        );

        // 仅当内容区因滚动条收窄时才重算(ring / 无滚动条时与上面完全一致,直接复用,省 4 次 Vec 分配)。
        if content_area != inner {
            (main_lengths, cross_lengths) = axis_lengths(content_area);
        }
        let (width, height) = content_size(
            layout_style.flex_direction,
            &main_lengths,
            &cross_lengths,
            layout_style.gap,
        );
        let justify_constraints = lengths_to_constraints(&main_lengths);
        let align_constraints = lengths_to_constraints(&cross_lengths);

        let rect = Rect::new(0, 0, width, height);
        drawer.push_scroll_buffer(Buffer::empty(rect));

        drawer.area = drawer.buffer_mut().area;

        // flex layout
        let layout = layout_style.get_layout().constraints(justify_constraints);
        let areas = layout.split(drawer.area);

        let mut new_areas: Vec<ratatui::prelude::Rect> = vec![];

        let rev_direction = cross_direction(layout_style.flex_direction);
        for (area, constraint) in areas.iter().zip(align_constraints.iter()) {
            let area = Layout::new(rev_direction, [constraint]).split(*area)[0];
            new_areas.push(area);
        }

        new_areas
    }

    fn draw(&mut self, drawer: &mut crate::ComponentDrawer<'_, '_>) {
        // 暂存外框供 calc_children_areas 的 `ring` 判定复用(与 render_ref 同一真源)。
        self.outer = Some(drawer.area);
        if let Some(block) = self.block.as_ref() {
            let inner_area = block.inner(drawer.area);
            drawer.render_widget(block, drawer.area);
            drawer.area = inner_area;
        }
    }
}

pub struct UseScrollImpl {
    scroll_view_state: State<ScrollViewState>,
    scrollbars: Scrollbars<'static>,
    // 组件外框(pre_component_draw 在 draw() 把 area 改成 inner 之前捕获)。
    outer: Option<ratatui::layout::Rect>,
    block: Option<Block<'static>>,
}

impl Hook for UseScrollImpl {
    fn pre_component_draw(&mut self, drawer: &mut crate::ComponentDrawer) {
        // 此刻 drawer.area 仍是组件完整区(draw() 尚未把它缩成 block.inner)。
        self.outer = Some(drawer.area);
    }
    fn post_component_draw(&mut self, drawer: &mut crate::ComponentDrawer) {
        // pop 本层内容缓冲(嵌套安全:guard 避免 unwrap on None);pop 后 buffer_mut 回到外层。
        let Some(buffer) = drawer.pop_scroll_buffer() else {
            return;
        };
        let outer = self.outer.unwrap_or_default();
        // inner 与 draw() 用同一 block.inner(),对部分边框/padding/标题一致。
        let inner = self
            .block
            .as_ref()
            .map(|block| block.inner(outer))
            .unwrap_or(outer);

        self.scrollbars.render_ref(
            outer,
            inner,
            drawer.buffer_mut(),
            &mut self.scroll_view_state.write_no_update(),
            &buffer,
        );
    }
}
