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

use unicode_width::UnicodeWidthStr;

use super::{
    layout::{resolve_column_widths, visible_column_indices, visible_columns},
    render::{RenderTable, RenderedCell, RenderedRow, render_table, rendered_rows_height},
    state::{TableState, sync_default_selection},
    types::{
        HighlightSpacing, RenderTableRow, TableBorderMode, TableCell, TableColumn, TableWrapMode,
    },
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
    /// Optional summary/footer row cells, one per column. Empty = no footer.
    pub footer: Vec<TableCell>,
    pub state: Option<State<TableState>>,
    pub active: bool,
    pub default_index: Option<usize>,
    pub on_select: Handler<'static, T>,
    pub block: Option<Block<'static>>,
    pub header_style: Style,
    pub footer_style: Style,
    pub row_style: Style,
    pub highlight_style: Style,
    /// Applied to every cell of the column referenced by `TableState::selected_column`.
    pub column_highlight_style: Style,
    /// Applied to the intersection of the selected row and the selected column.
    pub cell_highlight_style: Style,
    pub highlight_symbol: Option<&'static str>,
    pub highlight_spacing: HighlightSpacing,
    /// When `true`, an active table moves the selected column with Left/Right.
    pub column_navigation: bool,
    pub column_spacing: u16,
    pub wrap_mode: TableWrapMode,
    pub border_mode: TableBorderMode,
    pub border_style: Style,
    pub horizontal_line_style: Style,
    pub cell_padding: u16,
    pub header_separator: bool,
    pub footer_separator: bool,
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
            footer: Vec::new(),
            state: None,
            active: true,
            default_index: None,
            on_select: Handler::default(),
            block: None,
            header_style: Style::new().fg(Color::Cyan),
            footer_style: Style::new().fg(Color::Cyan),
            row_style: Style::default(),
            highlight_style: Style::new().fg(Color::Black).bg(Color::Cyan),
            column_highlight_style: Style::default(),
            cell_highlight_style: Style::default(),
            highlight_symbol: Some("▶ "),
            highlight_spacing: HighlightSpacing::default(),
            column_navigation: false,
            column_spacing: 1,
            wrap_mode: TableWrapMode::Wrap,
            border_mode: TableBorderMode::Outer,
            border_style: Style::new().fg(Color::DarkGray),
            horizontal_line_style: Style::new().fg(Color::DarkGray),
            cell_padding: 1,
            header_separator: true,
            footer_separator: true,
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
    footer: Vec<TableCell>,
    state: Option<State<TableState>>,
    block: Option<Block<'static>>,
    header_style: Style,
    footer_style: Style,
    row_style: Style,
    highlight_style: Style,
    column_highlight_style: Style,
    cell_highlight_style: Style,
    highlight_symbol: Option<&'static str>,
    highlight_spacing: HighlightSpacing,
    column_spacing: u16,
    wrap_mode: TableWrapMode,
    border_mode: TableBorderMode,
    border_style: Style,
    horizontal_line_style: Style,
    cell_padding: u16,
    header_separator: bool,
    footer_separator: bool,
    row_separator: bool,
}

impl<T> Table<T>
where
    T: Clone + Send + Sync + Unpin + 'static,
{
    /// Single source of truth for turning props into the persistent component;
    /// used by `Component::new`, `from_props`, and the height estimator so the
    /// field list is never duplicated.
    fn build(props: &TableProps<T>, state: Option<State<TableState>>) -> Self {
        Self {
            columns: props.columns.clone(),
            rows: props.rows.clone(),
            render_row: props.render_row.clone(),
            footer: props.footer.clone(),
            state,
            block: props.block.clone(),
            header_style: props.header_style,
            footer_style: props.footer_style,
            row_style: props.row_style,
            highlight_style: props.highlight_style,
            column_highlight_style: props.column_highlight_style,
            cell_highlight_style: props.cell_highlight_style,
            highlight_symbol: props.highlight_symbol,
            highlight_spacing: props.highlight_spacing,
            column_spacing: props.column_spacing,
            wrap_mode: props.wrap_mode,
            border_mode: props.border_mode,
            border_style: props.border_style,
            horizontal_line_style: props.horizontal_line_style,
            cell_padding: props.cell_padding,
            header_separator: props.header_separator,
            footer_separator: props.footer_separator,
            row_separator: props.row_separator,
        }
    }

    fn from_props(props: &TableProps<T>, state: State<TableState>) -> Self {
        Self::build(props, Some(state))
    }

    /// Reserved leading width for the selection symbol, honoring `highlight_spacing`.
    fn gutter(&self, has_selection: bool) -> u16 {
        match self.highlight_symbol {
            Some(symbol) if self.highlight_spacing.should_reserve(has_selection) => {
                symbol.width() as u16
            }
            _ => 0,
        }
    }

    /// Patch the column/cell highlight onto the cells of the selected column.
    /// `visible_indices` maps each rendered cell back to its original column index.
    fn apply_column_highlight(
        &self,
        cells: &mut [RenderedCell],
        visible_indices: &[usize],
        selected_column: Option<usize>,
        is_selected_row: bool,
    ) {
        let Some(selected_column) = selected_column else {
            return;
        };
        for (cell, &original_index) in cells.iter_mut().zip(visible_indices) {
            if original_index == selected_column {
                cell.extra_style = if is_selected_row {
                    self.column_highlight_style.patch(self.cell_highlight_style)
                } else {
                    self.column_highlight_style
                };
            }
        }
    }

    fn render_rows(
        &self,
        selected: Option<usize>,
        selected_column: Option<usize>,
        visible_columns: &[TableColumn],
        visible_indices: &[usize],
        widths: &[u16],
    ) -> Vec<RenderedRow> {
        let mut rendered_rows = Vec::new();

        let mut header_cells =
            render_header_cells(visible_columns, widths, self.header_style, self.wrap_mode);
        self.apply_column_highlight(&mut header_cells, visible_indices, selected_column, false);
        rendered_rows.push(RenderedRow {
            cells: header_cells,
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
            let mut cells = render_body_cells(cells, visible_columns, widths, self.wrap_mode);
            self.apply_column_highlight(&mut cells, visible_indices, selected_column, is_selected);
            rendered_rows.push(RenderedRow {
                cells,
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

        if !self.footer.is_empty() {
            if self.footer_separator {
                rendered_rows.push(RenderedRow::separator(self.horizontal_line_style));
            }
            let mut footer_cells =
                render_body_cells(self.footer.clone(), visible_columns, widths, self.wrap_mode);
            self.apply_column_highlight(&mut footer_cells, visible_indices, selected_column, false);
            rendered_rows.push(RenderedRow {
                cells: footer_cells,
                style: self.footer_style,
                selected: false,
            });
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
        Self::build(props, props.state)
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

        let column_count = props.columns.len();
        let selected_column = state.read().selected_column();
        hooks.use_effect(
            move || {
                if selected_column.is_some_and(|index| index >= column_count) {
                    state.write().clamp_column(column_count);
                }
            },
            (selected_column, column_count),
        );

        let active = props.active;
        let column_navigation = props.column_navigation;
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
                KeyCode::Char('h') | KeyCode::Left if column_navigation => {
                    state.write().previous_column(column_count);
                    EventResult::Consumed
                }
                KeyCode::Char('l') | KeyCode::Right if column_navigation => {
                    state.write().next_column(column_count);
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
        let selected_column = state.read().selected_column();
        let visible_columns = visible_columns(&self.columns, drawer.area.width);
        if visible_columns.is_empty() {
            return;
        }
        let visible_indices = visible_column_indices(&self.columns, drawer.area.width);

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

        let gutter = self.gutter(selected.is_some());
        let widths = resolve_column_widths(
            &visible_columns,
            area.width,
            self.border_mode,
            self.cell_padding,
            self.column_spacing,
            gutter,
        );
        if widths.is_empty() || widths.iter().all(|width| *width == 0) {
            return;
        }

        let rendered_rows = self.render_rows(
            selected,
            selected_column,
            &visible_columns,
            &visible_indices,
            &widths,
        );

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
            gutter,
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
    let visible_indices = visible_column_indices(&props.columns, width);

    // 选中态无法在 update 阶段得知,而选中会引入 gutter 使列变窄、换行增多;
    // 故只要符号“可能”出现(spacing != Never)就按有 gutter 保守估高,避免运行时被裁。
    let gutter = estimate_gutter(props);
    let widths = resolve_column_widths(
        &visible_columns,
        area_width,
        props.border_mode,
        props.cell_padding,
        props.column_spacing,
        gutter,
    );
    if widths.is_empty() || widths.iter().all(|width| *width == 0) {
        return block_vertical_border(props.block.as_ref());
    }

    let table = Table::<T>::build(props, None);
    let rows = table.render_rows(None, None, &visible_columns, &visible_indices, &widths);

    rendered_rows_height(&rows, props.border_mode)
        .saturating_add(block_vertical_border(props.block.as_ref()))
}

/// The gutter width to assume during height estimation. Reserves the symbol
/// width whenever the symbol could appear (`highlight_spacing != Never`).
fn estimate_gutter<T>(props: &TableProps<T>) -> u16
where
    T: Clone + Send + Sync + Unpin + 'static,
{
    match props.highlight_symbol {
        Some(symbol) if props.highlight_spacing != HighlightSpacing::Never => symbol.width() as u16,
        _ => 0,
    }
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
            extra_style: Style::default(),
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
                extra_style: Style::default(),
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
            // 隔离 gutter 影响,单验换行 + grid 的高度累计。
            highlight_symbol: None,
            ..Default::default()
        };

        assert_eq!(estimate_table_height(&props, Some(10)), 10);
    }

    #[test]
    fn estimated_height_reserves_gutter_for_highlight_symbol() {
        // 相同的窄表:预留选中符号 gutter 会挤窄列宽、增加换行,估高必然 > 不预留(Never)。
        let make = |spacing: HighlightSpacing| TableProps::<RowData> {
            columns: vec![TableColumn::new("Name", Constraint::Length(4))],
            rows: vec![RowData("中文English混排")],
            render_row: Some(Arc::new(|row, _| vec![TableCell::new(row.0)])),
            block: Some(Block::bordered()),
            border_mode: TableBorderMode::Grid,
            wrap_mode: TableWrapMode::Wrap,
            highlight_symbol: Some("▶ "),
            highlight_spacing: spacing,
            ..Default::default()
        };

        assert!(
            estimate_table_height(&make(HighlightSpacing::WhenSelected), Some(10))
                > estimate_table_height(&make(HighlightSpacing::Never), Some(10))
        );
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
