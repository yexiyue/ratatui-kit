pub struct ComponentDrawer<'a, 'b: 'a> {
    pub frame: &'a mut ratatui::Frame<'b>,
    pub area: ratatui::layout::Rect,
}
