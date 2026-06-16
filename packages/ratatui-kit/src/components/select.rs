//! Select 组件：带键盘事件处理的单选列表。

use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Alignment, Constraint},
    style::{Color, Style},
    text::Line,
    widgets::{List, ListItem, ListState},
};
use ratatui_kit_macros::{Props, component, element, with_layout_style};

use super::list_state::sync_default_selection;
use crate::{
    AnyElement, Handler, Hooks, State, UseEffect, UseEventHandler, UseState,
    components::{Border, Center, Text, TextParagraph},
    input::{EventPriority, EventResult, EventScope},
};

#[with_layout_style(margin, offset, width, height)]
#[derive(Props)]
pub struct SelectProps<T>
where
    T: Into<ListItem<'static>> + Clone + Send + Sync + 'static,
{
    pub items: Vec<T>,
    pub on_select: Handler<'static, T>,
    pub state: Option<State<ListState>>,
    pub top_title: Option<Line<'static>>,
    pub bottom_title: Option<Line<'static>>,
    pub active: bool,
    pub default_index: Option<usize>,
    pub empty_message: TextParagraph<'static>,
    pub highlight_symbol: Option<&'static str>,
    pub style: Style,
    pub border_style: Style,
    pub highlight_style: Style,
    pub empty_style: Style,
    pub empty_width: Constraint,
    pub empty_height: Constraint,
}

impl<T> Default for SelectProps<T>
where
    T: Into<ListItem<'static>> + Clone + Send + Sync,
{
    fn default() -> Self {
        Self {
            items: Vec::new(),
            on_select: Handler::default(),
            state: None,
            top_title: None,
            bottom_title: None,
            active: true,
            default_index: None,
            empty_message: TextParagraph::from("No data"),
            highlight_symbol: None,
            style: Style::default(),
            border_style: Style::default(),
            highlight_style: Style::default().fg(Color::Black).bg(Color::Cyan),
            empty_style: Style::default().fg(Color::Yellow),
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
pub fn Select<T>(props: &mut SelectProps<T>, mut hooks: Hooks) -> impl Into<AnyElement<'static>>
where
    T: Into<ListItem<'static>> + Clone + Send + Sync + 'static,
{
    let state = hooks.use_state(ListState::default);
    let state = props.state.unwrap_or(state);

    let default_index = props.default_index;
    let item_count = props.items.len();
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
            KeyCode::Enter => {
                let selected_index = state.read().selected();
                if let Some(index) = selected_index
                    && let Some(item) = items.get(index)
                {
                    on_select(item.clone());
                }
                EventResult::Consumed
            }
            _ => EventResult::Ignored,
        }
    });

    let is_empty = props.items.is_empty();
    let mut list = List::new(props.items.clone())
        .style(props.style)
        .highlight_style(props.highlight_style);

    if let Some(highlight_symbol) = props.highlight_symbol {
        list = list.highlight_symbol(highlight_symbol);
    }

    element!(Border(
        margin: props.margin,
        offset: props.offset,
        width: props.width,
        height: props.height,
        border_style: props.border_style,
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
                    style: props.empty_style,
                    wrap: true,
                )
            }
        } else {
            stateful(list, state)
        }
    })
}
