use ratatui::layout::Constraint;

use super::types::{TableBorderMode, TableColumn};

pub(super) fn visible_columns(columns: &[TableColumn], table_width: u16) -> Vec<TableColumn> {
    columns
        .iter()
        .filter(|column| {
            column
                .min_table_width
                .is_none_or(|min_width| table_width >= min_width)
        })
        .cloned()
        .collect()
}

pub(super) fn resolve_column_widths(
    columns: &[TableColumn],
    area_width: u16,
    border_mode: TableBorderMode,
    cell_padding: u16,
    column_spacing: u16,
) -> Vec<u16> {
    let column_count = columns.len() as u16;
    if column_count == 0 {
        return Vec::new();
    }

    let reserved = match border_mode {
        TableBorderMode::Outer | TableBorderMode::Grid => column_count.saturating_add(1),
        TableBorderMode::None => column_spacing.saturating_mul(column_count.saturating_sub(1)),
    }
    .saturating_add(cell_padding.saturating_mul(2).saturating_mul(column_count));
    let available = area_width.saturating_sub(reserved).max(column_count) as u32;

    let mut fixed = 0u32;
    let mut fill_units = 0u32;
    let mut fill_indexes = Vec::new();
    let mut widths = vec![0u16; columns.len()];

    for (index, column) in columns.iter().enumerate() {
        match column.width {
            Constraint::Length(width) | Constraint::Min(width) | Constraint::Max(width) => {
                widths[index] = width;
                fixed += u32::from(width);
            }
            Constraint::Percentage(percent) => {
                let width = available
                    .saturating_mul(u32::from(percent))
                    .saturating_div(100) as u16;
                widths[index] = width;
                fixed += u32::from(width);
            }
            Constraint::Ratio(num, den) => {
                let width = if den == 0 {
                    0
                } else {
                    available.saturating_mul(num).saturating_div(den) as u16
                };
                widths[index] = width;
                fixed += u32::from(width);
            }
            Constraint::Fill(unit) => {
                fill_units += u32::from(unit.max(1));
                fill_indexes.push((index, u32::from(unit.max(1))));
            }
        }
    }

    let remaining = available.saturating_sub(fixed);
    for (index, unit) in fill_indexes {
        widths[index] = if fill_units == 0 {
            0
        } else {
            remaining.saturating_mul(unit).saturating_div(fill_units) as u16
        };
    }

    let total: u32 = widths.iter().map(|width| u32::from(*width)).sum();
    if total > available {
        let mut remaining = available;
        for (index, width) in widths.iter_mut().enumerate() {
            if index == columns.len() - 1 {
                *width = remaining as u16;
            } else {
                let scaled = (u32::from(*width) * available / total).max(1);
                *width = scaled as u16;
                remaining = remaining.saturating_sub(scaled);
            }
        }
    }

    widths
}

#[cfg(test)]
mod tests {
    use ratatui::layout::Constraint;

    use super::*;

    #[test]
    fn min_table_width_filters_columns() {
        let columns = vec![
            TableColumn::new("A", Constraint::Length(1)),
            TableColumn::new("B", Constraint::Length(1)).min_table_width(80),
        ];

        assert_eq!(visible_columns(&columns, 79).len(), 1);
        assert_eq!(visible_columns(&columns, 80).len(), 2);
    }

    #[test]
    fn percentage_and_fill_widths_share_available_width() {
        let columns = vec![
            TableColumn::new("A", Constraint::Percentage(50)),
            TableColumn::new("B", Constraint::Fill(1)),
            TableColumn::new("C", Constraint::Fill(1)),
        ];

        let widths = resolve_column_widths(&columns, 30, TableBorderMode::None, 0, 0);

        assert_eq!(widths, vec![15, 7, 7]);
    }
}
