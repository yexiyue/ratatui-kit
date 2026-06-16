//! VirtualList 组件：基于 `tui-widget-list` 的虚拟列表。

use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    style::Style,
    widgets::{Block, Widget},
};
use ratatui_kit::{
    Component, Handler, Props, State, UseEffect, UseEventHandler, UseState,
    input::{EventPriority, EventResult, EventScope},
    with_layout_style,
};
use tui_widget_list::{
    ListBuildContext, ListBuilder, ListState, ListView as TuiListView, ScrollAxis, ScrollDirection,
};

type VirtualItemRenderer<'a, W> = dyn Fn(&ListBuildContext) -> (W, u16) + Send + Sync + 'a;

/// 虚拟列表 item 渲染器。返回值是 `(widget, main_axis_size)`。
pub struct RenderVirtualItem<'a, W>(Option<Box<VirtualItemRenderer<'a, W>>>);

impl<W> RenderVirtualItem<'_, W> {
    fn call(&self, context: &ListBuildContext) -> (W, u16) {
        self.0
            .as_ref()
            .expect("VirtualList requires render_item when item_count is non-zero")(context)
    }

    fn is_default(&self) -> bool {
        self.0.is_none()
    }

    fn take(&mut self) -> Self {
        std::mem::take(self)
    }
}

impl<W> Default for RenderVirtualItem<'_, W> {
    fn default() -> Self {
        Self(None)
    }
}

impl<F, W> From<F> for RenderVirtualItem<'_, W>
where
    F: Fn(&ListBuildContext) -> (W, u16) + Send + Sync + 'static,
{
    fn from(f: F) -> Self {
        Self(Some(Box::new(f)))
    }
}

#[with_layout_style(margin, offset, width, height)]
#[derive(Props)]
pub struct VirtualListProps<W>
where
    W: Widget + 'static,
{
    pub state: Option<State<ListState>>,
    pub item_count: usize,
    pub render_item: RenderVirtualItem<'static, W>,
    pub active: bool,
    pub default_index: Option<usize>,
    pub on_select: Handler<'static, usize>,
    pub scroll_axis: ScrollAxis,
    pub scroll_direction: ScrollDirection,
    pub style: Style,
    pub block: Option<Block<'static>>,
    pub scroll_padding: u16,
    pub infinite_scrolling: bool,
}

impl<W> Default for VirtualListProps<W>
where
    W: Widget + 'static,
{
    fn default() -> Self {
        Self {
            state: None,
            item_count: 0,
            render_item: RenderVirtualItem::default(),
            active: true,
            default_index: None,
            on_select: Handler::default(),
            scroll_axis: ScrollAxis::Vertical,
            scroll_direction: ScrollDirection::Forward,
            style: Style::default(),
            block: None,
            scroll_padding: 0,
            infinite_scrolling: true,
            margin: Default::default(),
            offset: Default::default(),
            width: Default::default(),
            height: Default::default(),
        }
    }
}

fn sync_default_selection(
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
    } else if state.selected.is_none()
        && let Some(index) = valid_default
    {
        state.select(Some(index));
    }
}

/// 基于虚拟渲染的列表组件。
pub struct VirtualList<W>
where
    W: Widget + 'static,
{
    state: Option<State<ListState>>,
    item_count: usize,
    render_item: RenderVirtualItem<'static, W>,
    scroll_axis: ScrollAxis,
    scroll_direction: ScrollDirection,
    style: Style,
    block: Option<Block<'static>>,
    scroll_padding: u16,
    infinite_scrolling: bool,
}

impl<W> VirtualList<W>
where
    W: Widget + 'static,
{
    fn from_props(props: &mut VirtualListProps<W>) -> Self {
        Self {
            state: props.state,
            item_count: props.item_count,
            render_item: props.render_item.take(),
            scroll_axis: props.scroll_axis,
            scroll_direction: props.scroll_direction,
            style: props.style,
            block: props.block.clone(),
            scroll_padding: props.scroll_padding,
            infinite_scrolling: props.infinite_scrolling,
        }
    }
}

impl<W> Component for VirtualList<W>
where
    W: Widget + 'static,
{
    type Props<'a>
        = VirtualListProps<W>
    where
        Self: 'a;

    fn new(props: &Self::Props<'_>) -> Self {
        Self {
            state: props.state,
            item_count: props.item_count,
            render_item: RenderVirtualItem::default(),
            scroll_axis: props.scroll_axis,
            scroll_direction: props.scroll_direction,
            style: props.style,
            block: props.block.clone(),
            scroll_padding: props.scroll_padding,
            infinite_scrolling: props.infinite_scrolling,
        }
    }

    fn update(
        &mut self,
        props: &mut Self::Props<'_>,
        mut hooks: ratatui_kit::Hooks,
        updater: &mut ratatui_kit::ComponentUpdater,
    ) {
        let layout_style = props.layout_style();
        let mut hooks = hooks.with_context_stack(updater.component_context_stack());
        let local_state = hooks.use_state(ListState::default);
        let state = props.state.unwrap_or(local_state);

        let default_index = props.default_index;
        let item_count = props.item_count;
        let mut last_default_index = hooks.use_state(|| None::<Option<usize>>);
        hooks.use_effect(
            move || {
                let mut last_default = last_default_index.get();
                sync_default_selection(
                    &mut state.write(),
                    &mut last_default,
                    default_index,
                    item_count,
                );
                last_default_index.set(last_default);
            },
            (default_index, item_count),
        );

        let selected_index = state.read().selected;
        hooks.use_effect(
            move || {
                if selected_index.is_some_and(|index| index >= item_count) {
                    state.write().select(item_count.checked_sub(1));
                }
            },
            (selected_index, item_count),
        );

        let active = props.active;
        let mut on_select = props.on_select.take();
        hooks.use_event_handler(EventScope::Current, EventPriority::Normal, move |event| {
            if !active || item_count == 0 {
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
                    state.write().next();
                    EventResult::Consumed
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    state.write().previous();
                    EventResult::Consumed
                }
                KeyCode::Home => {
                    state.write().select(Some(0));
                    EventResult::Consumed
                }
                KeyCode::End => {
                    state.write().select(item_count.checked_sub(1));
                    EventResult::Consumed
                }
                KeyCode::Enter => {
                    let selected = state.read().selected;
                    if let Some(index) = selected {
                        on_select(index);
                    }
                    EventResult::Consumed
                }
                _ => EventResult::Ignored,
            }
        });

        updater.set_layout_style(layout_style);

        *self = Self {
            state: Some(state),
            ..Self::from_props(props)
        };
    }

    fn draw(&mut self, drawer: &mut ratatui_kit::ComponentDrawer<'_, '_>) {
        if self.render_item.is_default() && self.item_count > 0 {
            if let Some(block) = self.block.clone() {
                drawer.render_widget(block, drawer.area);
            }
            return;
        }

        let render_item = self.render_item.take();
        let builder = ListBuilder::new(|context: &ListBuildContext| render_item.call(context));

        let mut list = TuiListView::new(builder, self.item_count)
            .style(self.style)
            .scroll_axis(self.scroll_axis)
            .scroll_direction(self.scroll_direction)
            .scroll_padding(self.scroll_padding)
            .infinite_scrolling(self.infinite_scrolling);

        if let Some(block) = self.block.clone() {
            list = list.block(block);
        }

        if let Some(state) = &mut self.state {
            drawer.render_stateful_widget(list, drawer.area, &mut state.write_no_update());
        }
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
        assert_eq!(state.selected, None);

        sync_default_selection(&mut state, &mut last_default, Some(1), 3);
        assert_eq!(state.selected, Some(1));
    }

    #[test]
    fn default_selection_preserves_existing_cursor_when_items_change() {
        let mut state = ListState::default();
        state.select(Some(2));
        let mut last_default = Some(Some(0));

        sync_default_selection(&mut state, &mut last_default, Some(0), 5);
        assert_eq!(state.selected, Some(2));
    }

    #[test]
    fn default_selection_applies_changed_default() {
        let mut state = ListState::default();
        state.select(Some(2));
        let mut last_default = Some(Some(0));

        sync_default_selection(&mut state, &mut last_default, Some(1), 5);
        assert_eq!(state.selected, Some(1));
    }
}
