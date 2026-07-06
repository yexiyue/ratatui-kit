use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{StatefulWidget, Widget},
};

pub struct ComponentDrawer<'a, 'b: 'a> {
    pub area: ratatui::layout::Rect,
    pub frame: &'a mut ratatui::Frame<'b>,
    // 滚动缓冲栈:每个 ScrollView 进入子绘制前 push 自己的内容缓冲,退出后 pop。
    // 用栈(而非单槽)才能正确支持 ScrollView 嵌套——内层 pop 后 `buffer_mut`
    // 自动回到外层缓冲,外层不会因内层 take 走而拿到 None。
    pub scroll_buffers: Vec<Buffer>,
}

impl<'a, 'b> ComponentDrawer<'a, 'b> {
    pub fn new(frame: &'a mut ratatui::Frame<'b>, area: ratatui::layout::Rect) -> Self {
        Self {
            area,
            frame,
            scroll_buffers: Vec::new(),
        }
    }

    pub fn buffer_mut(&mut self) -> &mut ratatui::buffer::Buffer {
        if let Some(scroll_buffer) = self.scroll_buffers.last_mut() {
            scroll_buffer
        } else {
            self.frame.buffer_mut()
        }
    }

    /// Push a fresh content buffer for a `ScrollView` about to draw its children.
    pub fn push_scroll_buffer(&mut self, buffer: Buffer) {
        self.scroll_buffers.push(buffer);
    }

    /// Pop this `ScrollView`'s content buffer after its children have drawn.
    /// Returns `None` if there is nothing to pop (guards against nesting bugs).
    pub fn pop_scroll_buffer(&mut self) -> Option<Buffer> {
        self.scroll_buffers.pop()
    }

    pub fn render_widget<W: Widget>(&mut self, widget: W, area: Rect) {
        widget.render(area, self.buffer_mut());
    }

    pub fn render_stateful_widget<W: StatefulWidget>(
        &mut self,
        widget: W,
        area: Rect,
        state: &mut W::State,
    ) {
        widget.render(area, self.buffer_mut(), state);
    }
}
