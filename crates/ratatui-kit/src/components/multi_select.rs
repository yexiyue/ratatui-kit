// MultiSelect 组件：带键盘事件处理的多选列表。

use std::collections::HashSet;

use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Alignment, Constraint},
    style::Style,
    text::Line,
    widgets::{List, ListItem, ListState},
};
use ratatui_kit_macros::{Props, component, element, with_layout_style};

use super::list_state::sync_default_selection;
use crate::{
    AnyElement, ComponentTheme, Handler, Hooks, Palette, State, UseEffect, UseEventHandler,
    UseState, UseTheme,
    components::theme::resolve_style,
    components::{Border, Center, Text, TextParagraph},
    input::{EventPriority, EventResult, EventScope},
};

/// MultiSelect 组件的主题 slot。高亮为「`on_accent` 前景 + `selection` 底」;已勾选项取 `accent`。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MultiSelectTheme {
    /// 列表项常规样式。
    pub style: Style,
    /// 边框样式。
    pub border_style: Style,
    /// 光标所在项高亮样式。
    pub highlight_style: Style,
    /// 已勾选项样式。
    pub selected_item_style: Style,
    /// 空态提示样式。
    pub empty_style: Style,
}

impl ComponentTheme for MultiSelectTheme {
    fn from_palette(palette: &Palette) -> Self {
        Self {
            style: Style::new().fg(palette.fg),
            border_style: Style::new().fg(palette.border),
            highlight_style: Style::new().fg(palette.on_accent).bg(palette.selection),
            selected_item_style: Style::new().fg(palette.accent),
            empty_style: Style::new().fg(palette.warning),
        }
    }
}

impl Default for MultiSelectTheme {
    fn default() -> Self {
        Self::from_palette(&Palette::default())
    }
}

#[with_layout_style(margin, offset, width, height)]
#[derive(Props)]
pub struct MultiSelectProps<T>
where
    T: Into<ListItem<'static>> + Clone + Send + Sync + 'static,
{
    pub items: Vec<T>,
    pub on_change: Handler<'static, Vec<T>>,
    pub on_select: Handler<'static, Vec<T>>,
    pub state: Option<State<ListState>>,
    pub selected: Option<State<HashSet<usize>>>,
    pub top_title: Option<Line<'static>>,
    pub bottom_title: Option<Line<'static>>,
    pub active: bool,
    pub default_index: Option<usize>,
    pub empty_message: TextParagraph<'static>,
    pub highlight_symbol: Option<&'static str>,
    // 以下样式覆盖:`None` 用 `MultiSelectTheme`,`Some(s)` 以 `theme.patch(s)` 覆盖。
    pub style: Option<Style>,
    pub border_style: Option<Style>,
    pub highlight_style: Option<Style>,
    pub selected_item_style: Option<Style>,
    pub empty_style: Option<Style>,
    pub empty_width: Constraint,
    pub empty_height: Constraint,
}

impl<T> Default for MultiSelectProps<T>
where
    T: Into<ListItem<'static>> + Clone + Send + Sync,
{
    fn default() -> Self {
        Self {
            items: Vec::new(),
            on_change: Handler::default(),
            on_select: Handler::default(),
            state: None,
            selected: None,
            top_title: None,
            bottom_title: None,
            active: true,
            default_index: None,
            empty_message: TextParagraph::from("No data"),
            highlight_symbol: None,
            style: None,
            border_style: None,
            highlight_style: None,
            selected_item_style: None,
            empty_style: None,
            empty_width: Constraint::Percentage(50),
            empty_height: Constraint::Length(5),
            margin: Default::default(),
            offset: Default::default(),
            width: Default::default(),
            height: Default::default(),
        }
    }
}

#[component]
pub fn MultiSelect<T>(
    props: &mut MultiSelectProps<T>,
    mut hooks: Hooks,
) -> impl Into<AnyElement<'static>>
where
    T: Into<ListItem<'static>> + Clone + Send + Sync + 'static,
{
    let state = hooks.use_state(ListState::default);
    let state = props.state.unwrap_or(state);
    let selected = hooks.use_state(HashSet::<usize>::default);
    let selected = props.selected.unwrap_or(selected);

    let item_count = props.items.len();
    let default_index = props.default_index;
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

    hooks.use_effect(
        move || {
            selected.write().retain(|index| *index < item_count);
        },
        item_count,
    );

    let selected_index = state.read().selected();
    hooks.use_effect(
        move || {
            if selected_index.is_some_and(|index| index >= item_count) {
                state.write().select(item_count.checked_sub(1));
            }
        },
        (selected_index, item_count),
    );

    let active = props.active;
    let items = props.items.clone();
    let mut on_change = props.on_change.take();
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
                state.write().select_next();
                EventResult::Consumed
            }
            KeyCode::Char('k') | KeyCode::Up => {
                state.write().select_previous();
                EventResult::Consumed
            }
            KeyCode::Home => {
                state.write().select_first();
                EventResult::Consumed
            }
            KeyCode::End => {
                state.write().select_last();
                EventResult::Consumed
            }
            KeyCode::Char(' ') => {
                if let Some(index) = state.read().selected() {
                    let mut selected_set = selected.write();
                    if !selected_set.insert(index) {
                        selected_set.remove(&index);
                    }
                    drop(selected_set);
                    let changed_items = selected_items(&items, &selected.read());
                    on_change(changed_items);
                }
                EventResult::Consumed
            }
            KeyCode::Enter => {
                let chosen_items = selected_items(&items, &selected.read());
                on_select(chosen_items);
                EventResult::Consumed
            }
            _ => EventResult::Ignored,
        }
    });

    // 主题解析:每个 slot 铺底,对应 props 的 Option<Style> 在上 patch(None → 用主题)。
    let theme = hooks.use_component_theme::<MultiSelectTheme>();
    let style = resolve_style(theme.style, props.style);
    let border_style = resolve_style(theme.border_style, props.border_style);
    let highlight_style = resolve_style(theme.highlight_style, props.highlight_style);
    let selected_item_style = resolve_style(theme.selected_item_style, props.selected_item_style);
    let empty_style = resolve_style(theme.empty_style, props.empty_style);

    let is_empty = props.items.is_empty();
    let selected_snapshot = selected.read().clone();
    let list_items: Vec<ListItem<'static>> = props
        .items
        .clone()
        .into_iter()
        .enumerate()
        .map(|(index, item)| {
            let item: ListItem<'static> = item.into();
            if selected_snapshot.contains(&index) {
                item.style(selected_item_style)
            } else {
                item
            }
        })
        .collect();

    let mut list = List::new(list_items)
        .style(style)
        .highlight_style(highlight_style);

    if let Some(highlight_symbol) = props.highlight_symbol {
        list = list.highlight_symbol(highlight_symbol);
    }

    element!(Border(
        margin: props.margin,
        offset: props.offset,
        width: props.width,
        height: props.height,
        border_style: border_style,
        top_title: props.top_title.clone(),
        bottom_title: props.bottom_title.clone(),
    ) {
        if is_empty {
            Center(
                width: props.empty_width,
                height: props.empty_height,
            ) {
                Text(
                    text: props.empty_message.clone(),
                    alignment: Alignment::Center,
                    style: empty_style,
                    wrap: true,
                )
            }
        } else {
            stateful(list, state)
        }
    })
}

fn selected_items<T>(items: &[T], selected: &HashSet<usize>) -> Vec<T>
where
    T: Clone,
{
    items
        .iter()
        .enumerate()
        .filter(|(index, _)| selected.contains(index))
        .map(|(_, item)| item.clone())
        .collect()
}
