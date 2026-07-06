// Scrollbars 组件：滚动视图的滚动条配置与渲染，支持横向/纵向滚动条、可见性控制、自定义样式，
// 以及 `over_border` 开关(滚动条盖在 block 边框上还是退到框内)。
//
// ## 用法示例
// ```rust
// element!(ScrollView(
//     scrollbars: Scrollbars {
//         vertical_scrollbar_visibility: ScrollbarVisibility::Always,
//         horizontal_scrollbar_visibility: ScrollbarVisibility::Automatic,
//         over_border: false, // 想让滚动条退到边框内侧时设 false(默认 true 盖边框)
//         ..Default::default()
//     },
//     // ...
// ))
// ```
// 可灵活控制滚动条的显示策略和样式，适合长列表、表格、文档等场景。

use super::ScrollViewState;
use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect, Size},
    widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget},
};
use ratatui_kit_macros::Props;

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
// 滚动条可见性枚举。
pub enum ScrollbarVisibility {
    // 仅在需要时渲染滚动条。
    #[default]
    Automatic,
    // 始终渲染滚动条。
    Always,
    // 从不渲染滚动条（隐藏）。
    Never,
}

#[derive(Props, Clone, Hash)]
// 滚动条配置。
pub struct Scrollbars<'a> {
    // 纵向滚动条可见性。
    pub vertical_scrollbar_visibility: ScrollbarVisibility,
    // 横向滚动条可见性。
    pub horizontal_scrollbar_visibility: ScrollbarVisibility,
    // 纵向滚动条样式。
    pub vertical_scrollbar: Scrollbar<'a>,
    // 横向滚动条样式。
    pub horizontal_scrollbar: Scrollbar<'a>,
    /// 有 block 边框时,滚动条画在边框环上(不占内容区)还是退到 `block.inner()` 内
    /// (占用一行/一列)。默认 `true`(盖边框,观感更好);无 block 时退化为框内。
    pub over_border: bool,
}

impl Default for Scrollbars<'_> {
    fn default() -> Self {
        Self {
            vertical_scrollbar_visibility: ScrollbarVisibility::Automatic,
            horizontal_scrollbar_visibility: ScrollbarVisibility::Automatic,
            vertical_scrollbar: Scrollbar::new(ScrollbarOrientation::VerticalRight),
            horizontal_scrollbar: Scrollbar::new(ScrollbarOrientation::HorizontalBottom),
            over_border: true,
        }
    }
}

impl Scrollbars<'_> {
    // 单轴可见性:无「显示一条会挤占另一轴」的耦合时用(over_border 模式,滚动条在边框上不占内容)。
    fn axis_show(visibility: ScrollbarVisibility, space: i32) -> bool {
        match visibility {
            ScrollbarVisibility::Always => true,
            ScrollbarVisibility::Never => false,
            ScrollbarVisibility::Automatic => space < 0,
        }
    }

    /// 子节点布局所用区域:ring(over_border)模式 = 整个 inner;inset 模式 = inner 扣掉将显示的滚动条。
    /// 供 `calc_children_areas` 决定内容缓冲宽高。
    pub(crate) fn content_area(&self, inner: Rect, content: Size, ring: bool) -> Rect {
        if ring {
            return inner;
        }
        let horizontal_space = inner.width as i32 - content.width as i32;
        let vertical_space = inner.height as i32 - content.height as i32;
        let (show_horizontal, show_vertical) =
            self.visible_scrollbars(horizontal_space, vertical_space);
        Rect {
            width: inner.width.saturating_sub(show_vertical as u16),
            height: inner.height.saturating_sub(show_horizontal as u16),
            ..inner
        }
    }

    fn render_visible_area(&self, dst: Rect, buf: &mut Buffer, src: Rect, scroll_buffer: &Buffer) {
        for (src_row, dst_row) in src.rows().zip(dst.rows()) {
            for (src_col, dst_col) in src_row.columns().zip(dst_row.columns()) {
                buf[dst_col] = scroll_buffer[src_col].clone();
            }
        }
    }

    // 朝向内部固定,避免调用方设错朝向导致布局/渲染不一致(只放开符号/样式覆盖)。
    fn render_scrollbar(
        scrollbar: &Scrollbar<'_>,
        orientation: ScrollbarOrientation,
        area: Rect,
        buf: &mut Buffer,
        position: u16,
        content_len: u16,
        viewport_len: u16,
    ) {
        let hidden = content_len.saturating_sub(viewport_len);
        let mut scrollbar_state = ScrollbarState::new(hidden as usize).position(position as usize);
        scrollbar
            .clone()
            .orientation(orientation)
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

    /// 是否 ring(盖边框)模式:`over_border` 开启,且 block 在右侧与下方各留了一格边框可画。
    /// `calc_children_areas` 与 `render_ref` **共用同一判定**,避免两处发散(partial-border block)。
    pub(crate) fn ring(&self, outer: Rect, inner: Rect) -> bool {
        self.over_border && inner.right() < outer.right() && inner.bottom() < outer.bottom()
    }

    /// 渲染可见窗口 + 滚动条。
    ///
    /// - `outer`:组件外框(有 block 时含边框);`inner`:`block.inner()`(无 block 时 = outer)。
    /// - ring(盖边框)= `over_border` 且 inner 右/下方各有一格边框可画;此时视口 = 整个 inner,
    ///   滚动条画在边框环上、不占内容;否则 inset:视口 = inner 扣掉显示的滚动条。
    /// - 偏移量按**视口**裁剪(保证最后一行/列可达),`page_size` = 视口(供翻页/`is_at_bottom`)。
    pub fn render_ref(
        &self,
        outer: Rect,
        inner: Rect,
        buf: &mut Buffer,
        state: &mut ScrollViewState,
        scroll_buffer: &Buffer,
    ) {
        let content = scroll_buffer.area.as_size();
        let ring = self.ring(outer, inner);

        let horizontal_space = inner.width as i32 - content.width as i32;
        let vertical_space = inner.height as i32 - content.height as i32;
        let (show_horizontal, show_vertical) = if ring {
            // 边框上的滚动条不挤占内容,两轴独立按真实溢出判断,无角落耦合。
            (
                Self::axis_show(self.horizontal_scrollbar_visibility, horizontal_space),
                Self::axis_show(self.vertical_scrollbar_visibility, vertical_space),
            )
        } else {
            self.visible_scrollbars(horizontal_space, vertical_space)
        };

        // 视口:ring = 整个 inner;inset = inner 扣掉显示的滚动条。
        let viewport = if ring {
            inner.as_size()
        } else {
            Size::new(
                inner.width.saturating_sub(show_vertical as u16),
                inner.height.saturating_sub(show_horizontal as u16),
            )
        };

        // 按视口裁剪偏移:内容放得下时 `saturating_sub` 为 0,`min` 自然把偏移归零。
        let x = state
            .offset
            .x
            .min(content.width.saturating_sub(viewport.width));
        let y = state
            .offset
            .y
            .min(content.height.saturating_sub(viewport.height));
        state.offset = Position::new(x, y);
        state.size = Some(content);
        state.page_size = Some(viewport);

        // 把可见窗口 blit 到 inner 左上角(严格限制在 inner 内,无滚动条方向的边框得以保留)。
        let src = Rect::new(x, y, viewport.width, viewport.height).intersection(scroll_buffer.area);
        let dst = Rect::new(inner.x, inner.y, viewport.width, viewport.height);
        self.render_visible_area(dst, buf, src, scroll_buffer);

        // 滚动条位置:ring 时 viewport = inner,故 `inner.x + viewport.width == inner.right()`(边框环上);
        // inset 时即 inner 内最后一列/行。两种情形同一表达式,无需分支。
        if show_vertical {
            let area = Rect::new(inner.x + viewport.width, inner.y, 1, viewport.height);
            Self::render_scrollbar(
                &self.vertical_scrollbar,
                ScrollbarOrientation::VerticalRight,
                area,
                buf,
                y,
                content.height,
                viewport.height,
            );
        }
        if show_horizontal {
            let area = Rect::new(inner.x, inner.y + viewport.height, viewport.width, 1);
            Self::render_scrollbar(
                &self.horizontal_scrollbar,
                ScrollbarOrientation::HorizontalBottom,
                area,
                buf,
                x,
                content.width,
                viewport.width,
            );
        }
    }
}
