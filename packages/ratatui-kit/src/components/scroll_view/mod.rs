//! ScrollView 组件：可滚动视图容器，支持横向/纵向滚动条，适合长列表、文档阅读等场景。
//!
//! ## 用法示例
//!
//! ### 自动管理滚动状态（推荐）
//! ```rust
//! element!(ScrollView(
//!     scroll_bars: ScrollBars::default(),
//! ){
//!     // 子内容
//! })
//! ```
//!
//! ### 手动管理滚动状态
//! ```rust
//! let scroll_state = hooks.use_state(ScrollViewState::default);
//!
//! hooks.use_local_events(move |event| {
//!     scroll_state.write().handle_event(&event);
//! });
//!
//! element!(ScrollView(
//!     scroll_view_state: scroll_state,
//!     scroll_bars: ScrollBars::default(),
//! ){
//!     // 子内容
//! })
//! ```
//!
//! ScrollView 支持两种使用方式：
//! 1. 不传递 `scroll_view_state` 参数，组件会自动管理滚动状态
//! 2. 传递由 `use_state` 创建的 `scroll_view_state` 参数，手动管理滚动状态
//!
//! 当需要对滚动行为进行精确控制时（如程序化滚动、与其他状态联动等），建议使用手动管理模式。

use crate::{AnyElement, Component, layout_style::LayoutStyle};
use crate::{Hook, State, UseEvents, UseState};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect, Size},
    widgets::Block,
};
use ratatui_kit_macros::{Props, with_layout_style};
mod state;
pub use state::ScrollViewState;
mod scrollbars;
pub use scrollbars::{ScrollBars, ScrollbarVisibility};

#[with_layout_style]
#[derive(Default, Props)]
/// ScrollView 组件属性。
pub struct ScrollViewProps<'a> {
    /// 子元素列表。
    pub children: Vec<AnyElement<'a>>,
    /// 滚动条配置。
    pub scroll_bars: ScrollBars<'static>,
    /// 滚动状态。
    pub scroll_view_state: Option<State<ScrollViewState>>,

    /// 可选边框块。
    pub block: Option<Block<'static>>,

    pub disabled: bool,
}

/// ScrollView 组件实现。
pub struct ScrollView {
    scroll_bars: ScrollBars<'static>,
    block: Option<Block<'static>>,
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
            scroll_bars: props.scroll_bars.clone(),
            block: props.block.clone(),
        }
    }

    fn update(
        &mut self,
        props: &mut Self::Props<'_>,
        mut hooks: crate::Hooks,
        updater: &mut crate::ComponentUpdater,
    ) {
        let layout_style = props.layout_style();

        let this_scroll_view_state = hooks.use_state(ScrollViewState::default);

        let disabled = props.disabled;
        self.block = props.block.clone();

        {
            let hook = hooks.use_hook(|| UseScrollImpl {
                scroll_view_state: props.scroll_view_state.unwrap_or(this_scroll_view_state),
                scrollbars: props.scroll_bars.clone(),
                area: None,
                has_block: props.block.is_some(),
            });
            hook.scroll_view_state = props.scroll_view_state.unwrap_or(this_scroll_view_state);
            hook.scrollbars = props.scroll_bars.clone();
            hook.has_block = props.block.is_some();
        }

        hooks.use_local_events({
            let props_scroll_view_state = props.scroll_view_state;
            move |event| {
                if props_scroll_view_state.is_none() && !disabled {
                    this_scroll_view_state.write().handle_event(&event);
                }
            }
        });

        self.scroll_bars = props.scroll_bars.clone();

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

        let old_width_height = {
            let area = drawer.area;
            let (main_lengths, cross_lengths) = axis_lengths(area);
            content_size(
                layout_style.flex_direction,
                &main_lengths,
                &cross_lengths,
                layout_style.gap,
            )
        };

        let scrollbar_layout = self.scroll_bars.layout_for(
            drawer.area,
            Size::new(old_width_height.0, old_width_height.1),
        );

        let (width, height, justify_constraints, align_constraints) = {
            let area = scrollbar_layout.visible_area;
            let (main_lengths, cross_lengths) = axis_lengths(area);
            let (width, height) = content_size(
                layout_style.flex_direction,
                &main_lengths,
                &cross_lengths,
                layout_style.gap,
            );
            (
                width,
                height,
                lengths_to_constraints(&main_lengths),
                lengths_to_constraints(&cross_lengths),
            )
        };

        let rect = Rect::new(0, 0, width, height);
        drawer.scroll_buffer = Some(Buffer::empty(rect));

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
        if let Some(block) = self.block.as_ref() {
            let inner_area = block.inner(drawer.area);
            drawer.render_widget(block, drawer.area);
            drawer.area = inner_area;
        }
    }
}

pub struct UseScrollImpl {
    scroll_view_state: State<ScrollViewState>,
    scrollbars: ScrollBars<'static>,
    area: Option<ratatui::layout::Rect>,
    has_block: bool,
}

impl Hook for UseScrollImpl {
    fn pre_component_draw(&mut self, drawer: &mut crate::ComponentDrawer) {
        self.area = Some(if self.has_block {
            Rect {
                x: drawer.area.x + 1,
                y: drawer.area.y + 1,
                width: drawer.area.width.saturating_sub(1),
                height: drawer.area.height.saturating_sub(2),
            }
        } else {
            drawer.area
        });
    }
    fn post_component_draw(&mut self, drawer: &mut crate::ComponentDrawer) {
        let buffer = drawer.scroll_buffer.take().unwrap();

        self.scrollbars.render_ref(
            self.area.unwrap_or_default(),
            drawer.buffer_mut(),
            &mut self.scroll_view_state.write_no_update(),
            &buffer,
        );
    }
}
