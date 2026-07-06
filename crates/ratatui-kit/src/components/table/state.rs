#[derive(Debug, Clone, Default)]
pub struct TableState {
    selected: Option<usize>,
    selected_column: Option<usize>,
}

impl TableState {
    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    pub fn select(&mut self, index: Option<usize>) {
        self.selected = index;
    }

    /// The currently highlighted column index (into the full column list), if any.
    pub fn selected_column(&self) -> Option<usize> {
        self.selected_column
    }

    /// Highlight a column by index, or clear the column highlight with `None`.
    pub fn select_column(&mut self, index: Option<usize>) {
        self.selected_column = index;
    }

    pub fn next_column(&mut self, len: usize) {
        if len == 0 {
            self.selected_column = None;
            return;
        }

        self.selected_column = Some(match self.selected_column {
            Some(index) => index.saturating_add(1).min(len - 1),
            None => 0,
        });
    }

    pub fn previous_column(&mut self, len: usize) {
        if len == 0 {
            self.selected_column = None;
            return;
        }

        self.selected_column = Some(match self.selected_column {
            Some(index) => index.saturating_sub(1),
            None => 0,
        });
    }

    pub fn clamp_column(&mut self, len: usize) {
        self.selected_column = self.selected_column.and_then(|index| {
            if len == 0 {
                None
            } else {
                Some(index.min(len - 1))
            }
        });
    }

    pub fn select_first(&mut self, len: usize) {
        self.selected = (len > 0).then_some(0);
    }

    pub fn select_last(&mut self, len: usize) {
        self.selected = len.checked_sub(1);
    }

    pub fn next(&mut self, len: usize) {
        if len == 0 {
            self.selected = None;
            return;
        }

        self.selected = Some(match self.selected {
            Some(index) => index.saturating_add(1).min(len - 1),
            None => 0,
        });
    }

    pub fn previous(&mut self, len: usize) {
        if len == 0 {
            self.selected = None;
            return;
        }

        self.selected = Some(match self.selected {
            Some(index) => index.saturating_sub(1),
            None => 0,
        });
    }

    pub fn clamp(&mut self, len: usize) {
        self.selected = self.selected.and_then(|index| {
            if len == 0 {
                None
            } else {
                Some(index.min(len - 1))
            }
        });
    }
}

pub(super) fn sync_default_selection(
    state: &mut TableState,
    last_default_index: &mut Option<Option<usize>>,
    default_index: Option<usize>,
    row_count: usize,
) {
    let default_changed = *last_default_index != Some(default_index);
    let valid_default = default_index.filter(|index| *index < row_count);

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
    fn table_state_navigation_is_clamped() {
        let mut state = TableState::default();
        state.next(3);
        assert_eq!(state.selected(), Some(0));
        state.next(3);
        assert_eq!(state.selected(), Some(1));
        state.next(3);
        state.next(3);
        assert_eq!(state.selected(), Some(2));
        state.previous(3);
        assert_eq!(state.selected(), Some(1));
        state.previous(3);
        state.previous(3);
        assert_eq!(state.selected(), Some(0));
    }

    #[test]
    fn table_state_column_navigation_is_clamped() {
        let mut state = TableState::default();
        assert_eq!(state.selected_column(), None);
        state.next_column(3);
        assert_eq!(state.selected_column(), Some(0));
        state.next_column(3);
        state.next_column(3);
        state.next_column(3);
        assert_eq!(state.selected_column(), Some(2));
        state.previous_column(3);
        assert_eq!(state.selected_column(), Some(1));
        state.clamp_column(1);
        assert_eq!(state.selected_column(), Some(0));
        state.clamp_column(0);
        assert_eq!(state.selected_column(), None);
    }

    #[test]
    fn table_state_empty_rows_clear_selection() {
        let mut state = TableState::default();
        state.select(Some(2));
        state.next(0);
        assert_eq!(state.selected(), None);
        state.select(Some(2));
        state.clamp(0);
        assert_eq!(state.selected(), None);
    }

    #[test]
    fn default_selection_reapplies_after_empty_load() {
        let mut state = TableState::default();
        let mut last_default = None;

        sync_default_selection(&mut state, &mut last_default, Some(1), 0);
        assert_eq!(state.selected(), None);

        sync_default_selection(&mut state, &mut last_default, Some(1), 3);
        assert_eq!(state.selected(), Some(1));
    }

    #[test]
    fn changed_default_selection_overrides_previous_selection() {
        let mut state = TableState::default();
        let mut last_default = None;

        sync_default_selection(&mut state, &mut last_default, Some(1), 3);
        assert_eq!(state.selected(), Some(1));

        sync_default_selection(&mut state, &mut last_default, Some(2), 3);
        assert_eq!(state.selected(), Some(2));
    }
}
