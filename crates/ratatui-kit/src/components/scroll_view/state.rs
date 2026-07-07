// ScrollViewState：滚动视图的状态管理结构，记录偏移量、尺寸、页大小等。
//
// 常与 ScrollView 组件配合使用，支持键盘/鼠标事件驱动的滚动。
//
// ## 用法示例
// ```rust
// let scroll_state = hooks.use_state(ScrollViewState::default);
// element!(ScrollView(scroll_view_state: scroll_state) { ... })
// // 在事件处理器中调用 `scroll_state.write().handle_event(&event)`。
// ```
// 支持上下左右/翻页/鼠标滚轮等多种滚动方式。

use crossterm::event::{Event, KeyCode, KeyEventKind, MouseEventKind};
use ratatui::layout::{Position, Rect, Size};

#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
// 滚动视图状态。
pub struct ScrollViewState {
    // 偏移量是滚动视图需要移动的行数和列数。
    pub(crate) offset: Position,
    // 滚动视图的尺寸。在第一次渲染调用前不会被设置。
    pub(crate) size: Option<Size>,
    // 滚动视图一页的尺寸。在第一次渲染调用前不会被设置。
    pub(crate) page_size: Option<Size>,
    // 每个直接子节点在内容缓冲中的区域(内容坐标),由 ScrollView 每帧记录。
    // 供 `scroll_to_index` 把某个子节点滚进视口——用于"选中项联动滚动"。
    pub(crate) child_areas: Vec<Rect>,
}

impl ScrollViewState {
    // 创建一个偏移量为 (0, 0) 的新滚动视图状态
    pub fn new() -> Self {
        Self::default()
    }

    // 创建一个带有指定偏移量的新滚动视图状态
    pub fn with_offset(offset: Position) -> Self {
        Self {
            offset,
            ..Default::default()
        }
    }

    // 设置滚动视图状态的偏移量
    pub const fn set_offset(&mut self, offset: Position) {
        self.offset = offset;
    }

    // 获取滚动视图状态的偏移量
    pub const fn offset(&self) -> Position {
        self.offset
    }

    // 向上滚动一行
    pub const fn scroll_up(&mut self) {
        self.offset.y = self.offset.y.saturating_sub(1);
    }

    // 向下滚动一行
    pub const fn scroll_down(&mut self) {
        self.offset.y = self.offset.y.saturating_add(1);
    }

    // 向下滚动一页
    pub fn scroll_page_down(&mut self) {
        let page_size = self.page_size.map_or(1, |size| size.height);
        // 我们减去 1 以确保页面之间有一行重叠
        self.offset.y = self.offset.y.saturating_add(page_size).saturating_sub(1);
    }

    // 向上滚动一页
    pub fn scroll_page_up(&mut self) {
        let page_size = self.page_size.map_or(1, |size| size.height);
        // 我们加上 1 以确保页面之间有一行重叠
        self.offset.y = self.offset.y.saturating_add(1).saturating_sub(page_size);
    }

    // 向左滚动一列
    pub const fn scroll_left(&mut self) {
        self.offset.x = self.offset.x.saturating_sub(1);
    }

    // 向右滚动一列
    pub const fn scroll_right(&mut self) {
        self.offset.x = self.offset.x.saturating_add(1);
    }

    // 滚动到缓冲区顶部
    pub const fn scroll_to_top(&mut self) {
        self.offset = Position::ORIGIN;
    }

    // 滚动到缓冲区底部
    pub fn scroll_to_bottom(&mut self) {
        // 渲染调用会调整偏移量以确保不会滚动到缓冲区末尾之后，所以这里可以将偏移量设置为最大值
        let bottom = self
            .size
            .map_or(u16::MAX, |size| size.height.saturating_sub(1));
        self.offset.y = bottom;
    }

    /// The content size (the full scrollable buffer). `None` before the first render.
    pub const fn size(&self) -> Option<Size> {
        self.size
    }

    /// The visible page size (viewport after scrollbars). `None` before the first render.
    pub const fn page_size(&self) -> Option<Size> {
        self.page_size
    }

    /// Whether the last content row is visible in the current page.
    ///
    /// Returns `true` before the first render (size unknown). Ported from upstream
    /// `tui-scrollview`; relies on `page_size` meaning the visible viewport.
    pub fn is_at_bottom(&self) -> bool {
        let Some(size) = self.size else {
            return true;
        };
        let bottom = size.height.saturating_sub(1);
        let page_size = self.page_size.map_or(1, |size| size.height);
        self.offset.y.saturating_add(page_size) > bottom
    }

    /// Scroll the vertical offset the minimum amount so the row range
    /// `[y, y + height)` is inside the visible page. No-op if already visible.
    ///
    /// The render pass re-clamps against the content, so this only needs to move
    /// the offset toward the target.
    pub fn scroll_to_visible(&mut self, y: u16, height: u16) {
        let page = self.page_size.map_or(u16::MAX, |size| size.height);
        let top = self.offset.y;
        let target_bottom = y.saturating_add(height);
        if y < top {
            // target starts above the viewport → align its top to the viewport top
            self.offset.y = y;
        } else if target_bottom > top.saturating_add(page) {
            // target ends below the viewport → align its bottom to the viewport bottom
            self.offset.y = target_bottom.saturating_sub(page);
        }
    }

    /// The content-buffer area of the direct child at `index` (in child order),
    /// as recorded by `ScrollView` on the last render. `None` before the first
    /// render or when `index` is out of range.
    pub fn child_area(&self, index: usize) -> Option<Rect> {
        self.child_areas.get(index).copied()
    }

    /// Scroll so the direct child at `index` (in child order) is visible.
    ///
    /// This is the "follow the selection" primitive: after a page moves its
    /// selection over a list of `ScrollView` children, call this with the
    /// selected index so the viewport tracks it. No-op if `index` is unknown
    /// (e.g. before the first render). Child geometry is selection-independent,
    /// so the last recorded layout is correct for the new selection.
    pub fn scroll_to_index(&mut self, index: usize) {
        if let Some(area) = self.child_area(index) {
            self.scroll_to_visible(area.y, area.height);
        }
    }

    /// Returns `true` if the event was a scroll input this state acted on.
    pub fn handle_event(&mut self, event: &Event) -> bool {
        match event {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Up | KeyCode::Char('k') => self.scroll_up(),
                KeyCode::Down | KeyCode::Char('j') => self.scroll_down(),
                KeyCode::Left | KeyCode::Char('h') => self.scroll_left(),
                KeyCode::Right | KeyCode::Char('l') => self.scroll_right(),
                KeyCode::PageUp => self.scroll_page_up(),
                KeyCode::PageDown => self.scroll_page_down(),
                KeyCode::Home => self.scroll_to_top(),
                KeyCode::End => self.scroll_to_bottom(),
                _ => return false,
            },
            Event::Mouse(event) => match event.kind {
                MouseEventKind::ScrollDown => self.scroll_down(),
                MouseEventKind::ScrollUp => self.scroll_up(),
                MouseEventKind::ScrollLeft => self.scroll_left(),
                MouseEventKind::ScrollRight => self.scroll_right(),
                _ => return false,
            },
            _ => return false,
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_at_bottom_requires_the_last_row_to_be_visible() {
        let mut state = ScrollViewState {
            offset: Position::new(0, 4),
            size: Some(Size::new(1, 10)),
            page_size: Some(Size::new(1, 5)),
            ..Default::default()
        };
        assert!(!state.is_at_bottom());
        state.offset.y = 5;
        assert!(state.is_at_bottom());
    }

    #[test]
    fn is_at_bottom_before_first_render() {
        let state = ScrollViewState::default();
        assert!(state.is_at_bottom());
    }

    #[test]
    fn scroll_to_index_brings_child_into_view() {
        let mut state = ScrollViewState {
            size: Some(Size::new(10, 20)),
            page_size: Some(Size::new(10, 3)),
            child_areas: (0..10).map(|y| Rect::new(0, y, 10, 1)).collect(),
            ..Default::default()
        };
        // child 8 sits at y=8 (below the 3-row viewport) → offset = 8 + 1 - 3
        state.scroll_to_index(8);
        assert_eq!(state.offset.y, 6);
        // out-of-range index is a no-op
        state.scroll_to_index(100);
        assert_eq!(state.offset.y, 6);
        // child already visible → no change
        state.scroll_to_index(7);
        assert_eq!(state.offset.y, 6);
    }

    #[test]
    fn scroll_to_visible_only_moves_when_outside_the_page() {
        let mut state = ScrollViewState {
            offset: Position::new(0, 2),
            size: Some(Size::new(1, 20)),
            page_size: Some(Size::new(1, 5)),
            ..Default::default()
        };
        // already visible (rows 2..7 shown, target row 3) → no change
        state.scroll_to_visible(3, 1);
        assert_eq!(state.offset.y, 2);
        // below the viewport (target row 9) → align its bottom to the viewport bottom
        state.scroll_to_visible(9, 1);
        assert_eq!(state.offset.y, 5); // 9 + 1 - 5
        // above the viewport (target row 1) → align its top
        state.scroll_to_visible(1, 1);
        assert_eq!(state.offset.y, 1);
    }
}
