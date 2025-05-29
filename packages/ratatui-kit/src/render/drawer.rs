pub struct ComponentDrawer<'a> {
    frame: &'a mut ratatui::Frame<'a>,
    area: ratatui::layout::Rect,
}
