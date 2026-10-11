use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect},
    widgets::{StatefulWidget, Widget},
};

#[derive(Clone, Copy)]
struct InputViewport {
    delta_x: i32,
    delta_y: i32,
    clip: Rect,
}

fn translated(rect: Rect, x: i32, y: i32) -> Rect {
    let left = (i32::from(rect.x) + x).clamp(0, i32::from(u16::MAX));
    let top = (i32::from(rect.y) + y).clamp(0, i32::from(u16::MAX));
    let right = (i32::from(rect.right()) + x).clamp(0, i32::from(u16::MAX));
    let bottom = (i32::from(rect.bottom()) + y).clamp(0, i32::from(u16::MAX));
    Rect::new(
        left as u16,
        top as u16,
        right.saturating_sub(left) as u16,
        bottom.saturating_sub(top) as u16,
    )
}

pub struct ComponentDrawer<'a, 'b: 'a> {
    pub area: ratatui::layout::Rect,
    pub frame: &'a mut ratatui::Frame<'b>,
    // 滚动缓冲栈:每个 ScrollView 进入子绘制前 push 自己的内容缓冲,退出后 pop。
    // 用栈(而非单槽)才能正确支持 ScrollView 嵌套——内层 pop 后 `buffer_mut`
    // 自动回到外层缓冲,外层不会因内层 take 走而拿到 None。
    scroll_buffers: Vec<Buffer>,
    input_viewports: Vec<InputViewport>,
}

impl<'a, 'b> ComponentDrawer<'a, 'b> {
    pub fn new(frame: &'a mut ratatui::Frame<'b>, area: ratatui::layout::Rect) -> Self {
        Self {
            area,
            frame,
            scroll_buffers: Vec::new(),
            input_viewports: vec![InputViewport {
                delta_x: 0,
                delta_y: 0,
                clip: area,
            }],
        }
    }

    pub fn buffer_mut(&mut self) -> &mut ratatui::buffer::Buffer {
        if let Some(scroll_buffer) = self.scroll_buffers.last_mut() {
            scroll_buffer
        } else {
            self.frame.buffer_mut()
        }
    }

    /// 为滚动容器压入内容缓冲与坐标变换。`viewport` 使用父绘制坐标,
    /// `offset` 必须是最终blit使用的已裁剪内容偏移；子内容坐标映射到屏幕并受所有祖先视口裁剪。
    pub fn push_scroll_buffer(&mut self, buffer: Buffer, viewport: Rect, offset: Position) {
        let parent = *self.input_viewports.last().unwrap();
        self.input_viewports.push(InputViewport {
            delta_x: parent.delta_x + i32::from(viewport.x) - i32::from(offset.x),
            delta_y: parent.delta_y + i32::from(viewport.y) - i32::from(offset.y),
            clip: parent
                .clip
                .intersection(translated(viewport, parent.delta_x, parent.delta_y)),
        });
        self.scroll_buffers.push(buffer);
    }

    pub(crate) fn input_stamp(&self) -> crate::input::Stamp {
        let viewport = self.input_viewports.last().unwrap();
        let mut stamp = crate::input::draw_clock_stamp(self.area);
        stamp.screen_area =
            viewport
                .clip
                .intersection(translated(self.area, viewport.delta_x, viewport.delta_y));
        stamp.delta_x = viewport.delta_x;
        stamp.delta_y = viewport.delta_y;
        stamp
    }

    /// Pop this `ScrollView`'s content buffer after its children have drawn.
    /// Returns `None` if there is nothing to pop (guards against nesting bugs).
    pub fn pop_scroll_buffer(&mut self) -> Option<Buffer> {
        let buffer = self.scroll_buffers.pop();
        if buffer.is_some() {
            self.input_viewports.pop();
        }
        buffer
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
