use ratatui::{buffer::Buffer, layout::Rect, style::Style, text::Line};
use unicode_width::UnicodeWidthChar;

use super::types::TableBorderMode;

#[derive(Debug)]
pub(super) struct RenderedCell {
    pub(super) lines: Vec<Line<'static>>,
}

#[derive(Debug)]
pub(super) struct RenderedRow {
    pub(super) cells: Vec<RenderedCell>,
    pub(super) style: Style,
    pub(super) selected: bool,
}

impl RenderedRow {
    pub(super) fn separator(style: Style) -> Self {
        Self {
            cells: Vec::new(),
            style,
            selected: false,
        }
    }

    pub(super) fn render_height(&self, border_mode: TableBorderMode) -> u16 {
        if self.is_separator() {
            return u16::from(matches!(border_mode, TableBorderMode::Grid));
        }

        self.cells
            .iter()
            .map(|cell| cell.lines.len() as u16)
            .max()
            .unwrap_or(1)
            .max(1)
    }

    fn is_separator(&self) -> bool {
        self.cells.is_empty()
    }
}

pub(super) fn rendered_rows_height(rows: &[RenderedRow], border_mode: TableBorderMode) -> u16 {
    let border_height = u16::from(matches!(
        border_mode,
        TableBorderMode::Outer | TableBorderMode::Grid
    ))
    .saturating_mul(2);
    rows.iter().fold(border_height, |height, row| {
        height.saturating_add(row.render_height(border_mode))
    })
}

pub(super) struct RenderTable<'a> {
    pub(super) area: Rect,
    pub(super) buf: &'a mut Buffer,
    pub(super) rows: &'a [RenderedRow],
    pub(super) widths: &'a [u16],
    pub(super) border_mode: TableBorderMode,
    pub(super) border_style: Style,
    pub(super) cell_padding: u16,
    pub(super) column_spacing: u16,
    pub(super) highlight_symbol: Option<&'static str>,
}

pub(super) fn render_table(mut table: RenderTable<'_>) {
    let mut y = table.area.y;

    if matches!(
        table.border_mode,
        TableBorderMode::Outer | TableBorderMode::Grid
    ) {
        let style = table.border_style;
        render_border_line(&mut table, y, '┌', '┬', '┐', style);
        y += 1;
    }

    for row in table.rows {
        if y >= table.area.bottom() {
            break;
        }

        if row.is_separator() {
            if matches!(table.border_mode, TableBorderMode::Grid) {
                render_border_line(&mut table, y, '├', '┼', '┤', row.style);
                y += 1;
            }
            continue;
        }

        let height = row.render_height(table.border_mode);

        for line_index in 0..height {
            if y >= table.area.bottom() {
                break;
            }
            render_row_line(&mut table, row, line_index as usize, y);
            y += 1;
        }
    }

    if y < table.area.bottom()
        && matches!(
            table.border_mode,
            TableBorderMode::Outer | TableBorderMode::Grid
        )
    {
        let style = table.border_style;
        render_border_line(&mut table, y, '└', '┴', '┘', style);
    }
}

fn render_border_line(
    table: &mut RenderTable<'_>,
    y: u16,
    left: char,
    mid: char,
    right: char,
    style: Style,
) {
    let mut x = table.area.x;
    put(table.buf, x, y, left, style);
    x += 1;
    for (index, width) in table.widths.iter().copied().enumerate() {
        for _ in 0..width.saturating_add(table.cell_padding.saturating_mul(2)) {
            put(table.buf, x, y, '─', style);
            x += 1;
        }
        if index + 1 < table.widths.len() {
            let separator = if matches!(table.border_mode, TableBorderMode::Grid) {
                mid
            } else {
                '─'
            };
            put(table.buf, x, y, separator, style);
            x += 1;
        }
    }
    put(table.buf, x, y, right, style);
}

fn render_row_line(table: &mut RenderTable<'_>, row: &RenderedRow, line_index: usize, y: u16) {
    let row_style = row.style;
    let mut x = table.area.x;

    match table.border_mode {
        TableBorderMode::None => {}
        TableBorderMode::Outer | TableBorderMode::Grid => {
            put(table.buf, x, y, '│', table.border_style);
            x += 1;
        }
    }

    if row.selected
        && let Some(symbol) = table.highlight_symbol
    {
        write_text(table.buf, x, y, symbol, row_style);
    }

    for (index, width) in table.widths.iter().copied().enumerate() {
        for _ in 0..table.cell_padding {
            put(table.buf, x, y, ' ', row_style);
            x += 1;
        }

        let line = row
            .cells
            .get(index)
            .and_then(|cell| cell.lines.get(line_index))
            .cloned()
            .unwrap_or_default();
        render_line(table.buf, x, y, width, line, row_style);
        x += width;

        for _ in 0..table.cell_padding {
            put(table.buf, x, y, ' ', row_style);
            x += 1;
        }

        if index + 1 < table.widths.len() {
            match table.border_mode {
                TableBorderMode::Grid => {
                    put(table.buf, x, y, '│', table.border_style);
                    x += 1;
                }
                TableBorderMode::Outer => {}
                TableBorderMode::None => {
                    for _ in 0..table.column_spacing {
                        put(table.buf, x, y, ' ', row_style);
                        x += 1;
                    }
                }
            }
        }
    }

    if matches!(
        table.border_mode,
        TableBorderMode::Outer | TableBorderMode::Grid
    ) {
        put(table.buf, x, y, '│', table.border_style);
    }
}

fn render_line(
    buf: &mut Buffer,
    x: u16,
    y: u16,
    width: u16,
    line: Line<'static>,
    row_style: Style,
) {
    let mut offset = 0u16;
    for span in line.spans {
        let style = row_style.patch(span.style);
        for c in span.content.chars() {
            let char_width = c.width().unwrap_or(0) as u16;
            if offset + char_width > width {
                return;
            }
            put(buf, x + offset, y, c, style);
            if char_width == 2 && offset + 1 < width {
                put(buf, x + offset + 1, y, ' ', style);
            }
            offset += char_width;
        }
    }

    while offset < width {
        put(buf, x + offset, y, ' ', row_style);
        offset += 1;
    }
}

fn write_text(buf: &mut Buffer, x: u16, y: u16, text: &str, style: Style) {
    let mut offset = 0u16;
    for c in text.chars() {
        put(buf, x + offset, y, c, style);
        offset += c.width().unwrap_or(0) as u16;
    }
}

fn put(buf: &mut Buffer, x: u16, y: u16, c: char, style: Style) {
    let cell = &mut buf[(x, y)];
    cell.set_char(c);
    cell.set_style(style);
}
