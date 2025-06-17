use crossterm::event::{Event, KeyCode, KeyEventKind, MouseEventKind};
use ratatui::layout::{Position, Size};

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
pub struct ScrollViewState {
    /// 偏移量是滚动视图需要移动的行数和列数。
    pub(crate) offset: Position,
    /// 滚动视图的尺寸。在第一次渲染调用前不会被设置。
    pub(crate) size: Option<Size>,
    /// 滚动视图一页的尺寸。在第一次渲染调用前不会被设置。
    pub(crate) page_size: Option<Size>,
}

impl ScrollViewState {
    /// 创建一个偏移量为 (0, 0) 的新滚动视图状态
    pub fn new() -> Self {
        Self::default()
    }

    /// 创建一个带有指定偏移量的新滚动视图状态
    pub fn with_offset(offset: Position) -> Self {
        Self {
            offset,
            ..Default::default()
        }
    }

    /// 设置滚动视图状态的偏移量
    pub const fn set_offset(&mut self, offset: Position) {
        self.offset = offset;
    }

    /// 获取滚动视图状态的偏移量
    pub const fn offset(&self) -> Position {
        self.offset
    }

    /// 向上滚动一行
    pub const fn scroll_up(&mut self) {
        self.offset.y = self.offset.y.saturating_sub(1);
    }

    /// 向下滚动一行
    pub const fn scroll_down(&mut self) {
        self.offset.y = self.offset.y.saturating_add(1);
    }

    /// 向下滚动一页
    pub fn scroll_page_down(&mut self) {
        let page_size = self.page_size.map_or(1, |size| size.height);
        // 我们减去 1 以确保页面之间有一行重叠
        self.offset.y = self.offset.y.saturating_add(page_size).saturating_sub(1);
    }

    /// 向上滚动一页
    pub fn scroll_page_up(&mut self) {
        let page_size = self.page_size.map_or(1, |size| size.height);
        // 我们加上 1 以确保页面之间有一行重叠
        self.offset.y = self.offset.y.saturating_add(1).saturating_sub(page_size);
    }

    /// 向左滚动一列
    pub const fn scroll_left(&mut self) {
        self.offset.x = self.offset.x.saturating_sub(1);
    }

    /// 向右滚动一列
    pub const fn scroll_right(&mut self) {
        self.offset.x = self.offset.x.saturating_add(1);
    }

    /// 滚动到缓冲区顶部
    pub const fn scroll_to_top(&mut self) {
        self.offset = Position::ORIGIN;
    }

    /// 滚动到缓冲区底部
    pub fn scroll_to_bottom(&mut self) {
        // 渲染调用会调整偏移量以确保不会滚动到缓冲区末尾之后，所以这里可以将偏移量设置为最大值
        let bottom = self
            .size
            .map_or(u16::MAX, |size| size.height.saturating_sub(1));
        self.offset.y = bottom;
    }

    pub fn handle_event(&mut self, event: &Event) {
        match event {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.scroll_up();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.scroll_down();
                }
                KeyCode::Left | KeyCode::Char('h') => {
                    self.scroll_left();
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    self.scroll_right();
                }
                KeyCode::PageUp => {
                    self.scroll_page_up();
                }
                KeyCode::PageDown => {
                    self.scroll_page_down();
                }
                KeyCode::Home => {
                    self.scroll_to_top();
                }
                KeyCode::End => {
                    self.scroll_to_bottom();
                }
                _ => {}
            },
            Event::Mouse(event) => match event.kind {
                MouseEventKind::ScrollDown => {
                    self.scroll_down();
                }
                MouseEventKind::ScrollUp => {
                    self.scroll_up();
                }
                MouseEventKind::ScrollLeft => {
                    self.scroll_left();
                }
                MouseEventKind::ScrollRight => {
                    self.scroll_right();
                }
                _ => {}
            },
            _ => {}
        }
    }
}
