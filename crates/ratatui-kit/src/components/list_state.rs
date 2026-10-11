use ratatui::{
    layout::{Margin, Position, Rect},
    widgets::ListState,
};

pub(crate) fn sync_default_selection(
    state: &mut ListState,
    last_default_index: &mut Option<Option<usize>>,
    default_index: Option<usize>,
    item_count: usize,
) {
    let default_changed = *last_default_index != Some(default_index);
    let valid_default = default_index.filter(|index| *index < item_count);

    if default_changed {
        *last_default_index = Some(default_index);
        state.select(valid_default);
    } else if state.selected().is_none()
        && let Some(index) = valid_default
    {
        state.select(Some(index));
    }
}

/// 按实际 ListItem 高度映射可见行;边框与视口尾部空白不属于条目。
pub(crate) fn item_at_position(
    area: Rect,
    position: Position,
    offset: usize,
    heights: &[usize],
) -> Option<usize> {
    let inner = area.inner(Margin::new(1, 1));
    if !inner.contains(position) {
        return None;
    }
    let mut row = usize::from(position.y - inner.y);
    let mut remaining = usize::from(inner.height);
    for (index, height) in heights.iter().enumerate().skip(offset) {
        // ratatui List 只绘制完整放入视口的条目。
        if *height > remaining {
            break;
        }
        remaining -= height;
        if row < *height {
            return Some(index);
        }
        row = row.saturating_sub(*height);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_hit_respects_offset_heights_and_empty_rows() {
        let area = Rect::new(10, 20, 8, 8);
        let heights = [3, 2, 1];
        assert_eq!(
            item_at_position(area, Position::new(11, 21), 1, &heights),
            Some(1)
        );
        assert_eq!(
            item_at_position(area, Position::new(11, 22), 1, &heights),
            Some(1)
        );
        assert_eq!(
            item_at_position(area, Position::new(11, 23), 1, &heights),
            Some(2)
        );
        assert_eq!(
            item_at_position(area, Position::new(11, 24), 1, &heights),
            None
        );
        assert_eq!(
            item_at_position(area, Position::new(10, 21), 1, &heights),
            None
        );
        assert_eq!(
            item_at_position(area, Position::new(11, 20), 1, &heights),
            None
        );
    }

    #[test]
    fn list_hit_matches_rendered_whole_items() {
        use ratatui::{
            buffer::Buffer,
            widgets::{Block, List, ListItem, StatefulWidget},
        };
        let area = Rect::new(0, 0, 12, 8);
        let items = vec![ListItem::new("a\na\na"), ListItem::new("b\nb\nb\nb")];
        let mut buffer = Buffer::empty(area);
        let mut state = ListState::default();
        List::new(items)
            .block(Block::bordered())
            .render(area, &mut buffer, &mut state);
        for y in 1..7 {
            let expected = (buffer[(1, y)].symbol() == "a").then_some(0);
            assert_eq!(
                item_at_position(area, Position::new(1, y), state.offset(), &[3, 4]),
                expected
            );
        }
    }

    #[test]
    fn default_selection_reapplies_after_empty_load() {
        let mut state = ListState::default();
        let mut last_default = None;

        sync_default_selection(&mut state, &mut last_default, Some(1), 0);
        assert_eq!(state.selected(), None);

        sync_default_selection(&mut state, &mut last_default, Some(1), 3);
        assert_eq!(state.selected(), Some(1));
    }

    #[test]
    fn default_selection_preserves_existing_cursor_when_items_change() {
        let mut state = ListState::default();
        state.select(Some(2));
        let mut last_default = Some(Some(0));

        sync_default_selection(&mut state, &mut last_default, Some(0), 5);
        assert_eq!(state.selected(), Some(2));
    }

    #[test]
    fn default_selection_applies_changed_default() {
        let mut state = ListState::default();
        state.select(Some(2));
        let mut last_default = Some(Some(0));

        sync_default_selection(&mut state, &mut last_default, Some(1), 5);
        assert_eq!(state.selected(), Some(1));
    }
}
