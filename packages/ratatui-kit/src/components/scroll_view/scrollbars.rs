//! ScrollBars 组件：滚动视图的滚动条配置与渲染，支持横向/纵向滚动条、可见性控制、自定义样式。
//!
//! ## 用法示例
//! ```rust
//! element!(ScrollView(
//!     scroll_bars: ScrollBars {
//!         vertical_scrollbar_visibility: ScrollbarVisibility::Always,
//!         horizontal_scrollbar_visibility: ScrollbarVisibility::Automatic,
//!         ..Default::default()
//!     },
//!     // ...
//! ))
//! ```
//! 可灵活控制滚动条的显示策略和样式，适合长列表、表格、文档等场景。

use super::ScrollViewState;
use ratatui::{
    buffer::Buffer,
    layout::{Rect, Size},
    widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget},
};
use ratatui_kit_macros::Props;

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
/// 滚动条可见性枚举。
pub enum ScrollbarVisibility {
    /// 仅在需要时渲染滚动条。
    #[default]
    Automatic,
    /// 始终渲染滚动条。
    Always,
    /// 从不渲染滚动条（隐藏）。
    Never,
}

#[derive(Props, Clone, Hash)]
/// 滚动条配置。
pub struct ScrollBars<'a> {
    /// 纵向滚动条可见性。
    pub vertical_scrollbar_visibility: ScrollbarVisibility,
    /// 横向滚动条可见性。
    pub horizontal_scrollbar_visibility: ScrollbarVisibility,
    /// 纵向滚动条样式。
    pub vertical_scrollbar: Scrollbar<'a>,
    /// 横向滚动条样式。
    pub horizontal_scrollbar: Scrollbar<'a>,
}

impl Default for ScrollBars<'_> {
    fn default() -> Self {
        Self {
            vertical_scrollbar_visibility: ScrollbarVisibility::Automatic,
            horizontal_scrollbar_visibility: ScrollbarVisibility::Automatic,
            vertical_scrollbar: Scrollbar::new(ScrollbarOrientation::VerticalRight),
            horizontal_scrollbar: Scrollbar::new(ScrollbarOrientation::HorizontalBottom),
        }
    }
}

impl ScrollBars<'_> {
    fn render_visible_area(
        &self,
        area: Rect,
        buf: &mut Buffer,
        visible_area: Rect,
        scroll_buffer: &Buffer,
    ) {
        // TODO: 这里可能有更高效的实现方式
        for (src_row, dst_row) in visible_area.rows().zip(area.rows()) {
            for (src_col, dst_col) in src_row.columns().zip(dst_row.columns()) {
                buf[dst_col] = scroll_buffer[src_col].clone();
            }
        }
    }

    fn render_vertical_scrollbar(
        &self,
        area: Rect,
        buf: &mut Buffer,
        state: &ScrollViewState,
        scroll_size: Size,
    ) {
        let scrollbar_height = scroll_size.height.saturating_sub(area.height);
        let mut scrollbar_state =
            ScrollbarState::new(scrollbar_height as usize).position(state.offset.y as usize);

        self.vertical_scrollbar
            .clone()
            .render(area, buf, &mut scrollbar_state);
    }

    fn render_horizontal_scrollbar(
        &self,
        area: Rect,
        buf: &mut Buffer,
        state: &ScrollViewState,
        scroll_size: Size,
    ) {
        let scrollbar_width = scroll_size.width.saturating_sub(area.width);

        let mut scrollbar_state =
            ScrollbarState::new(scrollbar_width as usize).position(state.offset.x as usize);
        self.horizontal_scrollbar
            .clone()
            .render(area, buf, &mut scrollbar_state);
    }

    pub fn visible_scrollbars(&self, horizontal_space: i32, vertical_space: i32) -> (bool, bool) {
        type V = ScrollbarVisibility;

        match (
            self.horizontal_scrollbar_visibility,
            self.vertical_scrollbar_visibility,
        ) {
            // 直接渲染，无需检查适配值
            (V::Always, V::Always) => (true, true),
            (V::Never, V::Never) => (false, false),
            (V::Always, V::Never) => (true, false),
            (V::Never, V::Always) => (false, true),

            // Auto => 仅在不适配时渲染滚动条
            (V::Automatic, V::Never) => (horizontal_space < 0, false),
            (V::Never, V::Automatic) => (false, vertical_space < 0),

            // Auto => 渲染滚动条如果：
            //   不适配；或
            //   完全适配（另一个滚动条占用一行导致触发）
            (V::Always, V::Automatic) => (true, vertical_space <= 0),
            (V::Automatic, V::Always) => (horizontal_space <= 0, true),

            // 仅依赖适配值
            (V::Automatic, V::Automatic) => {
                if horizontal_space >= 0 && vertical_space >= 0 {
                    // 两个方向都有足够空间
                    (false, false)
                } else if horizontal_space < 0 && vertical_space < 0 {
                    // 两个方向都没有足够空间
                    (true, true)
                } else if horizontal_space > 0 && vertical_space < 0 {
                    // 水平适配，垂直不适配
                    (false, true)
                } else if horizontal_space < 0 && vertical_space > 0 {
                    // 垂直适配，水平不适配
                    (true, false)
                } else {
                    // 一个方向完全适配，另一个方向不适配，导致两个滚动条都可见，因为另一个滚动条会占用缓冲区的一行
                    (true, true)
                }
            }
        }
    }

    fn render_scrollbars(
        &self,
        area: Rect,
        buf: &mut Buffer,
        state: &mut ScrollViewState,
        scroll_buffer: &Buffer,
    ) -> Rect {
        let size: ratatui::prelude::Size = scroll_buffer.area.as_size();
        // 每个方向的适配值
        //   > 0 => 适配
        //  == 0 => 完全适配
        //   < 0 => 不适配
        let horizontal_space = area.width as i32 - size.width as i32;
        let vertical_space = area.height as i32 - size.height as i32;

        // 如果该方向适配，则重置状态
        if horizontal_space > 0 {
            state.offset.x = 0;
        }
        if vertical_space > 0 {
            state.offset.y = 0;
        }

        let (show_horizontal, show_vertical) =
            self.visible_scrollbars(horizontal_space, vertical_space);

        let new_height = if show_horizontal {
            // 如果两个滚动条都渲染，避免角落重叠
            let width = area.width.saturating_sub(show_vertical as u16);
            let render_area = Rect { width, ..area };
            // 渲染滚动条，更新可用空间
            self.render_horizontal_scrollbar(render_area, buf, state, size);
            area.height.saturating_sub(1)
        } else {
            area.height
        };

        let new_width = if show_vertical {
            // 如果两个滚动条都渲染，避免角落重叠
            let height = area.height.saturating_sub(show_horizontal as u16);
            let render_area = Rect { height, ..area };
            // 渲染滚动条，更新可用空间
            self.render_vertical_scrollbar(render_area, buf, state, size);
            area.width.saturating_sub(1)
        } else {
            area.width
        };

        Rect::new(state.offset.x, state.offset.y, new_width, new_height)
    }

    pub fn render_ref(
        &self,
        area: Rect,
        buf: &mut Buffer,
        state: &mut ScrollViewState,
        scroll_buffer: &Buffer,
    ) {
        let (mut x, mut y) = state.offset.into();
        // 确保不会在任一方向上滚动超过缓冲区末尾
        let max_x_offset = scroll_buffer.area.width.saturating_sub(area.width);
        let max_y_offset = scroll_buffer.area.height.saturating_sub(area.height);

        x = x.min(max_x_offset);
        y = y.min(max_y_offset);
        state.offset = (x, y).into();
        state.size = Some(scroll_buffer.area.as_size());
        state.page_size = Some(area.into());
        let visible_area = self
            .render_scrollbars(area, buf, state, scroll_buffer)
            .intersection(scroll_buffer.area);
        self.render_visible_area(area, buf, visible_area, scroll_buffer);
    }
}
