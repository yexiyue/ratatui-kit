use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{StatefulWidget, StatefulWidgetRef, Widget, WidgetRef},
};

pub struct ComponentDrawer<'a, 'b: 'a> {
    pub area: ratatui::layout::Rect,
    pub frame: &'a mut ratatui::Frame<'b>,
    pub scroll_buffer: Option<Buffer>,
}

impl<'a, 'b> ComponentDrawer<'a, 'b> {
    pub fn new(frame: &'a mut ratatui::Frame<'b>, area: ratatui::layout::Rect) -> Self {
        Self {
            area,
            frame,
            scroll_buffer: None,
        }
    }

    pub fn buffer_mut(&mut self) -> &mut ratatui::buffer::Buffer {
        if let Some(scroll_buffer) = &mut self.scroll_buffer {
            scroll_buffer
        } else {
            self.frame.buffer_mut()
        }
    }

    pub fn render_widget<W: Widget>(&mut self, widget: W, area: Rect) {
        widget.render(area, self.buffer_mut());
    }

    pub fn render_widget_ref<W: WidgetRef>(&mut self, widget: W, area: Rect) {
        widget.render_ref(area, self.buffer_mut());
    }

    pub fn render_stateful_widget<W: StatefulWidget>(
        &mut self,
        widget: W,
        area: Rect,
        state: &mut W::State,
    ) {
        widget.render(area, self.buffer_mut(), state);
    }

    pub fn render_stateful_widget_ref<W: StatefulWidgetRef>(
        &mut self,
        widget: W,
        area: Rect,
        state: &mut W::State,
    ) {
        widget.render_ref(area, self.buffer_mut(), state);
    }
}
