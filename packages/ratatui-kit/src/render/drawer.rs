pub struct ComponentDrawer<'a> {
    pub frame: &'a mut ratatui::Frame<'a>,
    pub area: ratatui::layout::Rect,
}
