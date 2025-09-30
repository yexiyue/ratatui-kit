use ratatui::layout::{Constraint, Direction, Flex, Layout, Margin, Offset};

#[derive(Default, Clone)]
pub struct LayoutStyle {
    pub flex_direction: Direction,
    pub justify_content: Flex,
    pub gap: i32,
    pub margin: Margin,
    pub offset: Offset,
    pub width: Constraint,
    pub height: Constraint,
}

impl LayoutStyle {
    pub fn get_layout(&self) -> Layout {
        Layout::default()
            .direction(self.flex_direction)
            .flex(self.justify_content)
            .spacing(self.gap)
    }

    pub fn get_width(&self) -> Constraint {
        self.width
    }

    pub fn get_height(&self) -> Constraint {
        self.height
    }

    pub fn inner_area(&self, area: ratatui::layout::Rect) -> ratatui::layout::Rect {
        area.offset(self.offset).inner(self.margin)
    }
}
