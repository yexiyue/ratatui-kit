use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::Constraint,
    style::{Color, Style},
    widgets::Block,
};
use ratatui_kit::{
    Component, ComponentDrawer, ComponentUpdater, Handler, Hooks, Props, State, UseEffect,
    UseEventHandler, UseState,
    input::{EventPriority, EventResult, EventScope},
    with_layout_style,
};

use super::{
    layout::{resolve_column_widths, visible_columns},
    render::{RenderTable, RenderedCell, RenderedRow, render_table, rendered_rows_height},
    state::{TableState, sync_default_selection},
    types::{RenderTableRow, TableBorderMode, TableCell, TableColumn, TableWrapMode},
    wrap::wrap_line,
};

#[with_layout_style(margin, offset, width, height)]
#[derive(Props)]
pub struct TableProps<T>
where
    T: Clone + Send + Sync + Unpin + 'static,
{
    pub columns: Vec<TableColumn>,
    pub rows: Vec<T>,
    pub render_row: Option<RenderTableRow<T>>,
    pub state: Option<State<TableState>>,
    pub active: bool,
    pub default_index: Option<usize>,
    pub on_select: Handler<'static, T>,
    pub block: Option<Block<'static>>,
    pub header_style: Style,
    pub row_style: Style,
    pub highlight_style: Style,
    pub highlight_symbol: Option<&'static str>,
    pub column_spacing: u16,
    pub wrap_mode: TableWrapMode,
    pub border_mode: TableBorderMode,
    pub border_style: Style,
    pub horizontal_line_style: Style,
    pub cell_padding: u16,
    pub header_separator: bool,
    pub row_separator: bool,
}

impl<T> Default for TableProps<T>
where
    T: Clone + Send + Sync + Unpin + 'static,
{
    fn default() -> Self {
        Self {
            columns: Vec::new(),
            rows: Vec::new(),
            render_row: None,
            state: None,
            active: true,
            default_index: None,
            on_select: Handler::default(),
            block: None,
            header_style: Style::new().fg(Color::Cyan),
            row_style: Style::default(),
            highlight_style: Style::new().fg(Color::Black).bg(Color::Cyan),
            highlight_symbol: Some("▶ "),
            column_spacing: 1,
            wrap_mode: TableWrapMode::Wrap,
            border_mode: TableBorderMode::Outer,
            border_style: Style::new().fg(Color::DarkGray),
            horizontal_line_style: Style::new().fg(Color::DarkGray),
            cell_padding: 1,
            header_separator: true,
            row_separator: false,
            margin: Default::default(),
            offset: Default::default(),
            width: Default::default(),
            height: Default::default(),
        }
    }
}

pub struct Table<T>
where
    T: Clone + Send + Sync + Unpin + 'static,
{
    columns: Vec<TableColumn>,
    rows: Vec<T>,
    render_row: Option<RenderTableRow<T>>,
    state: Option<State<TableState>>,
    block: Option<Block<'static>>,
    header_style: Style,
    row_style: Style,
    highlight_style: Style,
    highlight_symbol: Option<&'static str>,
    column_spacing: u16,
    wrap_mode: TableWrapMode,
    border_mode: TableBorderMode,
    border_style: Style,
    horizontal_line_style: Style,
    cell_padding: u16,
    header_separator: bool,
    row_separator: bool,
}

impl<T> Table<T>
where
    T: Clone + Send + Sync + Unpin + 'static,
{
    fn from_props(props: &TableProps<T>, state: State<TableState>) -> Self {
        Self {
            columns: props.columns.clone(),
            rows: props.rows.clone(),
            render_row: props.render_row.clone(),
            state: Some(state),
            block: props.block.clone(),
            header_style: props.header_style,
            row_style: props.row_style,
            highlight_style: props.highlight_style,
            highlight_symbol: props.highlight_symbol,
            column_spacing: props.column_spacing,
            wrap_mode: props.wrap_mode,
            border_mode: props.border_mode,
            border_style: props.border_style,
            horizontal_line_style: props.horizontal_line_style,
            cell_padding: props.cell_padding,
            header_separator: props.header_separator,
            row_separator: props.row_separator,
        }
    }
    fn render_rows(
        &self,
        selected: Option<usize>,
        visible_columns: &[TableColumn],
        widths: &[u16],
    ) -> Vec<RenderedRow> {
        let mut rendered_rows = Vec::new();
        rendered_rows.push(RenderedRow {
            cells: render_header_cells(visible_columns, widths, self.header_style, self.wrap_mode),
            style: self.header_style,
            selected: false,
        });

        if self.header_separator {
            rendered_rows.push(RenderedRow::separator(self.horizontal_line_style));
        }

        for (index, item) in self.rows.iter().enumerate() {
            let is_selected = selected == Some(index);
            let cells = self
                .render_row
                .as_ref()
                .map(|render_row| render_row(item, is_selected))
                .unwrap_or_default();
            rendered_rows.push(RenderedRow {
                cells: render_body_cells(cells, visible_columns, widths, self.wrap_mode),
                style: if is_selected {
                    self.row_style.patch(self.highlight_style)
                } else {
                    self.row_style
                },
                selected: is_selected,
            });
            if self.row_separator && index + 1 < self.rows.len() {
                rendered_rows.push(RenderedRow::separator(self.horizontal_line_style));
            }
        }

        rendered_rows
    }
}

impl<T> Component for Table<T>
where
    T: Clone + Send + Sync + Unpin + 'static,
{
    type Props<'a>
        = TableProps<T>
    where
        Self: 'a;

    fn new(props: &Self::Props<'_>) -> Self {
        Self {
            columns: props.columns.clone(),
            rows: props.rows.clone(),
            render_row: props.render_row.clone(),
            state: props.state,
            block: props.block.clone(),
            header_style: props.header_style,
            row_style: props.row_style,
            highlight_style: props.highlight_style,
            highlight_symbol: props.highlight_symbol,
            column_spacing: props.column_spacing,
            wrap_mode: props.wrap_mode,
            border_mode: props.border_mode,
            border_style: props.border_style,
            horizontal_line_style: props.horizontal_line_style,
            cell_padding: props.cell_padding,
            header_separator: props.header_separator,
            row_separator: props.row_separator,
        }
    }

    fn update(
        &mut self,
        props: &mut Self::Props<'_>,
        mut hooks: Hooks,
        updater: &mut ComponentUpdater,
    ) {
        let layout_style = props.layout_style();
        let estimated_height = estimate_table_height(props, None);
        let mut layout_style = layout_style;
        if layout_style.height == Constraint::Percentage(100) {
            layout_style.height = Constraint::Length(estimated_height);
        }
        let mut hooks = hooks.with_context_stack(updater.component_context_stack());
        let local_state = hooks.use_state(TableState::default);
        let state = props.state.unwrap_or(local_state);

        let default_index = props.default_index;
        let row_count = props.rows.len();
        let mut last_default_index = hooks.use_state(|| None::<Option<usize>>);
        hooks.use_effect(
            move || {
                let mut last_default = last_default_index.get();
                sync_default_selection(
                    &mut state.write(),
                    &mut last_default,
                    default_index,
                    row_count,
                );
                last_default_index.set(last_default);
            },
            (default_index, row_count),
        );

        let selected_index = state.read().selected();
        hooks.use_effect(
            move || {
                if selected_index.is_some_and(|index| index >= row_count) {
                    state.write().clamp(row_count);
                }
            },
            (selected_index, row_count),
        );

        let active = props.active;
        let rows = props.rows.clone();
        let mut on_select = props.on_select.take();
        hooks.use_event_handler(EventScope::Current, EventPriority::Normal, move |event| {
            if !active || row_count == 0 {
                return EventResult::Ignored;
            }

            let Event::Key(key) = event else {
                return EventResult::Ignored;
            };
            if key.kind != KeyEventKind::Press {
                return EventResult::Ignored;
            }

            match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    state.write().next(row_count);
                    EventResult::Consumed
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    state.write().previous(row_count);
                    EventResult::Consumed
                }
                KeyCode::Home => {
                    state.write().select_first(row_count);
                    EventResult::Consumed
                }
                KeyCode::End => {
                    state.write().select_last(row_count);
                    EventResult::Consumed
                }
                KeyCode::Enter => {
                    if let Some(index) = state.read().selected()
                        && let Some(row) = rows.get(index)
                    {
                        on_select(row.clone());
                    }
                    EventResult::Consumed
                }
                _ => EventResult::Ignored,
            }
        });

        updater.set_layout_style(layout_style);
        *self = Self::from_props(props, state);
    }

    fn draw(&mut self, drawer: &mut ComponentDrawer<'_, '_>) {
        let Some(state) = &mut self.state else {
            return;
        };

        let selected = state.read().selected();
        let visible_columns = visible_columns(&self.columns, drawer.area.width);
        if visible_columns.is_empty() {
            return;
        }

        let area = if let Some(block) = self.block.clone() {
            let inner = block.inner(drawer.area);
            drawer.render_widget(block, drawer.area);
            inner
        } else {
            drawer.area
        };
        if area.is_empty() {
            return;
        }

        let widths = resolve_column_widths(
            &visible_columns,
            area.width,
            self.border_mode,
            self.cell_padding,
            self.column_spacing,
        );
        if widths.is_empty() || widths.iter().all(|width| *width == 0) {
            return;
        }

        let rendered_rows = self.render_rows(selected, &visible_columns, &widths);

        render_table(RenderTable {
            area,
            buf: drawer.buffer_mut(),
            rows: &rendered_rows,
            widths: &widths,
            border_mode: self.border_mode,
            border_style: self.border_style,
            cell_padding: self.cell_padding,
            column_spacing: self.column_spacing,
            highlight_symbol: self.highlight_symbol,
        });
    }
}

fn estimate_table_height<T>(props: &TableProps<T>, area_width: Option<u16>) -> u16
where
    T: Clone + Send + Sync + Unpin + 'static,
{
    let width = area_width.unwrap_or_else(|| constraint_hint_width(props.width));
    let area_width = width.saturating_sub(block_horizontal_border(props.block.as_ref()));
    let visible_columns = visible_columns(&props.columns, width);
    if visible_columns.is_empty() {
        return block_vertical_border(props.block.as_ref());
    }

    let widths = resolve_column_widths(
        &visible_columns,
        area_width,
        props.border_mode,
        props.cell_padding,
        props.column_spacing,
    );
    if widths.is_empty() || widths.iter().all(|width| *width == 0) {
        return block_vertical_border(props.block.as_ref());
    }

    let table = Table::<T> {
        columns: props.columns.clone(),
        rows: props.rows.clone(),
        render_row: props.render_row.clone(),
        state: None,
        block: props.block.clone(),
        header_style: props.header_style,
        row_style: props.row_style,
        highlight_style: props.highlight_style,
        highlight_symbol: props.highlight_symbol,
        column_spacing: props.column_spacing,
        wrap_mode: props.wrap_mode,
        border_mode: props.border_mode,
        border_style: props.border_style,
        horizontal_line_style: props.horizontal_line_style,
        cell_padding: props.cell_padding,
        header_separator: props.header_separator,
        row_separator: props.row_separator,
    };
    let rows = table.render_rows(None, &visible_columns, &widths);

    rendered_rows_height(&rows, props.border_mode)
        .saturating_add(block_vertical_border(props.block.as_ref()))
}

fn constraint_hint_width(width: Constraint) -> u16 {
    match width {
        Constraint::Length(width) | Constraint::Min(width) | Constraint::Max(width) => width,
        Constraint::Percentage(percent) => percent.max(1),
        Constraint::Ratio(num, den) => {
            if den == 0 {
                num.max(1) as u16
            } else {
                num.saturating_mul(100).saturating_div(den).max(1) as u16
            }
        }
        Constraint::Fill(_) => 80,
    }
}

fn block_horizontal_border(block: Option<&Block<'static>>) -> u16 {
    block
        .map(|block| {
            let outer = ratatui::layout::Rect::new(0, 0, 10, 3);
            let inner = block.inner(outer);
            outer.width.saturating_sub(inner.width)
        })
        .unwrap_or_default()
}

fn block_vertical_border(block: Option<&Block<'static>>) -> u16 {
    block
        .map(|block| {
            let outer = ratatui::layout::Rect::new(0, 0, 10, 3);
            let inner = block.inner(outer);
            outer.height.saturating_sub(inner.height)
        })
        .unwrap_or_default()
}

fn render_header_cells(
    columns: &[TableColumn],
    widths: &[u16],
    style: Style,
    wrap_mode: TableWrapMode,
) -> Vec<RenderedCell> {
    columns
        .iter()
        .zip(widths.iter().copied())
        .map(|(column, width)| RenderedCell {
            lines: wrap_line(
                column.header.clone().style(style),
                width as usize,
                wrap_mode,
                column.alignment,
            ),
        })
        .collect()
}

fn render_body_cells(
    cells: Vec<TableCell>,
    columns: &[TableColumn],
    widths: &[u16],
    wrap_mode: TableWrapMode,
) -> Vec<RenderedCell> {
    (0..columns.len())
        .map(|index| {
            let cell = cells
                .get(index)
                .cloned()
                .unwrap_or_else(|| TableCell::new(""));
            let alignment = cell.alignment.unwrap_or(columns[index].alignment);
            RenderedCell {
                lines: wrap_line(
                    cell.line.style(cell.style),
                    widths.get(index).copied().unwrap_or_default() as usize,
                    wrap_mode,
                    alignment,
                ),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ratatui::{layout::Constraint, widgets::Block};

    use super::*;

    #[derive(Clone)]
    struct RowData(&'static str);

    #[test]
    fn estimated_height_accounts_for_wrapped_rows_and_grid() {
        let props = TableProps::<RowData> {
            columns: vec![TableColumn::new("Name", Constraint::Length(4))],
            rows: vec![RowData("中文English混排")],
            render_row: Some(Arc::new(|row, _| vec![TableCell::new(row.0)])),
            block: Some(Block::bordered()),
            border_mode: TableBorderMode::Grid,
            wrap_mode: TableWrapMode::Wrap,
            ..Default::default()
        };

        assert_eq!(estimate_table_height(&props, Some(10)), 10);
    }

    #[test]
    fn explicit_height_is_preserved_by_layout_style() {
        let props = TableProps::<RowData> {
            height: Constraint::Length(3),
            ..Default::default()
        };

        assert_eq!(props.layout_style().height, Constraint::Length(3));
    }
}
