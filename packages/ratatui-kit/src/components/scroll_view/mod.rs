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

use std::sync::{Arc, RwLock};

use crate::{AnyElement, Component, layout_style::LayoutStyle};
use crate::{Hook, State, UseEffect, UseEvents, UseState};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
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
}

/// ScrollView 组件实现。
pub struct ScrollView {
    scroll_bars: ScrollBars<'static>,
    scroll_view_state: Arc<RwLock<ScrollViewState>>,
}

impl Component for ScrollView {
    type Props<'a> = ScrollViewProps<'a>;

    fn new(props: &Self::Props<'_>) -> Self {
        Self {
            scroll_bars: props.scroll_bars.clone(),
            scroll_view_state: Arc::new(RwLock::new(ScrollViewState::default())),
        }
    }

    fn update(
        &mut self,
        props: &mut Self::Props<'_>,
        mut hooks: crate::Hooks,
        updater: &mut crate::ComponentUpdater,
    ) {
        let layout_style = props.layout_style();

        let scrollbars = hooks.use_state(|| props.scroll_bars.clone());

        let mut update_flag = hooks.use_state(|| false);

        hooks.use_effect(
            || {
                *scrollbars.write() = props.scroll_bars.clone();
            },
            props.scroll_bars.clone(),
        );

        if let Some(state) = &props.scroll_view_state {
            let state = state.get();
            *self.scroll_view_state.write().unwrap() = state;
        }

        hooks.use_hook(|| UseScrollImpl {
            scroll_view_state: self.scroll_view_state.clone(),
            scrollbars,
            area: None,
        });

        hooks.use_local_events({
            let scroll_view_state = self.scroll_view_state.clone();
            let props_scroll_view_state = props.scroll_view_state.clone();
            move |event| {
                if let Some(mut state) = props_scroll_view_state {
                    state.set(scroll_view_state.read().unwrap().clone());
                } else {
                    scroll_view_state.write().unwrap().handle_event(&event);
                    update_flag.set(!update_flag.get());
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
        let constraint_sum = |d: Direction, len: u16| {
            children
                .get_constraints(d)
                .iter()
                .map(|c| match c {
                    Constraint::Length(h) => *h,
                    Constraint::Percentage(p) => len * *p / 100,
                    Constraint::Ratio(r, n) => {
                        if *n != 0 {
                            len * (*r as u16) / (*n as u16)
                        } else {
                            0
                        }
                    }
                    Constraint::Min(min) => *min,
                    Constraint::Max(max) => *max,
                    Constraint::Fill(i) => len * i,
                })
                .collect::<Vec<_>>()
        };

        let old_width_height = {
            let area = drawer.area;
            match layout_style.flex_direction {
                Direction::Horizontal => {
                    let sum_w = constraint_sum(Direction::Horizontal, area.width);
                    let sum_count = sum_w.len();
                    let sum_w = sum_w.iter().sum::<u16>()
                        + ((sum_count as i32 - 1) * layout_style.gap) as u16;
                    let sum_h = constraint_sum(Direction::Vertical, area.height)
                        .into_iter()
                        .max()
                        .unwrap_or_default();
                    (sum_w, sum_h)
                }
                Direction::Vertical => {
                    let sum_h = constraint_sum(Direction::Vertical, area.height);
                    let sum_count = sum_h.len();
                    let sum_h = sum_h.iter().sum::<u16>()
                        + ((sum_count as i32 - 1) * layout_style.gap) as u16;
                    let sum_w = constraint_sum(Direction::Horizontal, area.width)
                        .into_iter()
                        .max()
                        .unwrap_or_default();
                    (sum_w, sum_h)
                }
            }
        };

        let horizontal_space = drawer.area.width as i32 - old_width_height.0 as i32 + 1;
        let vertical_space = drawer.area.height as i32 - old_width_height.1 as i32 + 1;
        let (show_horizontal, show_vertical) = self
            .scroll_bars
            .visible_scrollbars(horizontal_space, vertical_space);

        let (width, height, justify_constraints, align_constraints) = {
            let mut area = drawer.area;
            if show_horizontal {
                area.height -= 1;
            }
            if show_vertical {
                area.width -= 1;
            }
            match layout_style.flex_direction {
                Direction::Horizontal => {
                    let widths = constraint_sum(Direction::Horizontal, area.width);
                    let sum_count = widths.len();

                    let justify_constraints = widths
                        .iter()
                        .map(|c| Constraint::Length(*c))
                        .collect::<Vec<Constraint>>();

                    let sum_w = widths.iter().sum::<u16>()
                        + ((sum_count as i32 - 1) * layout_style.gap) as u16;

                    let heights = constraint_sum(Direction::Vertical, area.height);
                    let sum_h = heights.iter().max().copied().unwrap_or_default();

                    let align_constraints = heights
                        .iter()
                        .map(|c| Constraint::Length(*c))
                        .collect::<Vec<Constraint>>();

                    (sum_w, sum_h, justify_constraints, align_constraints)
                }
                Direction::Vertical => {
                    let heights = constraint_sum(Direction::Vertical, area.height);
                    let sum_count = heights.len();

                    let justify_constraints = heights
                        .iter()
                        .map(|c| Constraint::Length(*c))
                        .collect::<Vec<Constraint>>();

                    let sum_h = heights.iter().sum::<u16>()
                        + ((sum_count as i32 - 1) * layout_style.gap) as u16;

                    let widths = constraint_sum(Direction::Horizontal, area.width);
                    let sum_w = widths.iter().max().copied().unwrap_or_default();

                    let align_constraints = widths
                        .iter()
                        .map(|c| Constraint::Length(*c))
                        .collect::<Vec<Constraint>>();

                    (sum_w, sum_h, justify_constraints, align_constraints)
                }
            }
        };

        let rect = Rect::new(0, 0, width, height);
        drawer.scroll_buffer = Some(Buffer::empty(rect));

        drawer.area = drawer.buffer_mut().area;

        // flex layout
        let layout = layout_style.get_layout().constraints(justify_constraints);
        let areas = layout.split(drawer.area);

        let mut new_areas: Vec<ratatui::prelude::Rect> = vec![];

        let rev_direction = match layout_style.flex_direction {
            Direction::Horizontal => Direction::Vertical,
            Direction::Vertical => Direction::Horizontal,
        };
        for (area, constraint) in areas.iter().zip(align_constraints.iter()) {
            let area = Layout::new(rev_direction, [constraint]).split(*area)[0];
            new_areas.push(area);
        }

        new_areas
    }
}

pub struct UseScrollImpl {
    scroll_view_state: Arc<RwLock<ScrollViewState>>,
    scrollbars: State<ScrollBars<'static>>,
    area: Option<ratatui::layout::Rect>,
}

impl Hook for UseScrollImpl {
    fn pre_component_draw(&mut self, drawer: &mut crate::ComponentDrawer) {
        self.area = Some(drawer.area);
    }
    fn post_component_draw(&mut self, drawer: &mut crate::ComponentDrawer) {
        let buffer = drawer.scroll_buffer.take().unwrap();
        let scrollbars = self.scrollbars.read();
        let mut scroll_view_state = self.scroll_view_state.read().unwrap().clone();
        scrollbars.render_ref(
            self.area.unwrap_or_default(),
            drawer.buffer_mut(),
            &mut scroll_view_state,
            &buffer,
        );
        *self.scroll_view_state.write().unwrap() = scroll_view_state;
    }
}
