use ratatui::widgets::ListState;

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

#[cfg(test)]
mod tests {
    use super::*;

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
