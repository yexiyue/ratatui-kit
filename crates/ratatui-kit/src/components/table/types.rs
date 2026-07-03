use std::sync::Arc;

use ratatui::{layout::Constraint, style::Style, text::Line};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableWrapMode {
    Wrap,
    Truncate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableBorderMode {
    None,
    Outer,
    Grid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableCellAlignment {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone)]
pub struct TableColumn {
    pub header: Line<'static>,
    pub width: Constraint,
    pub alignment: TableCellAlignment,
    pub min_table_width: Option<u16>,
}

impl TableColumn {
    pub fn new(header: impl Into<Line<'static>>, width: Constraint) -> Self {
        Self {
            header: header.into(),
            width,
            alignment: TableCellAlignment::Left,
            min_table_width: None,
        }
    }

    pub fn alignment(mut self, alignment: TableCellAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    pub fn min_table_width(mut self, width: u16) -> Self {
        self.min_table_width = Some(width);
        self
    }
}

#[derive(Debug, Clone)]
pub struct TableCell {
    pub line: Line<'static>,
    pub style: Style,
    pub alignment: Option<TableCellAlignment>,
}

impl TableCell {
    pub fn new(line: impl Into<Line<'static>>) -> Self {
        Self {
            line: line.into(),
            style: Style::default(),
            alignment: None,
        }
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn alignment(mut self, alignment: TableCellAlignment) -> Self {
        self.alignment = Some(alignment);
        self
    }
}

impl<T> From<T> for TableCell
where
    T: Into<Line<'static>>,
{
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

pub type RenderTableRow<T> = Arc<dyn Fn(&T, bool) -> Vec<TableCell> + Send + Sync + 'static>;
