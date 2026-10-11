// MultiSelect 组件：带键盘事件处理的多选列表。

use std::{cell::RefCell, collections::HashSet, rc::Rc};

use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Alignment, Constraint},
    style::{Modifier, Style},
    text::Line,
    widgets::{List, ListItem, ListState},
};
use ratatui_kit_macros::{Props, component, element, with_layout_style};

use super::list_state::{item_at_position, sync_default_selection};
use crate::{
    AnyElement, ComponentTheme, Handler, Hooks, Palette, PointerKind, PointerOptions, State,
    UseEffect, UseEventHandler, UseState, UseTheme,
    components::theme::resolve_style,
    components::{Border, Center, Text, TextParagraph},
    hooks::use_pointer::{pointer_enabled, use_pointer_with_layout_sync},
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
    /// 悬停行样式(光标行/已勾选行优先级更高)。
    pub hover_style: Style,
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
            hover_style: Style::new()
                .bg(palette.surface)
                .fg(palette.fg)
                .add_modifier(Modifier::BOLD),
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
    pub hover_style: Option<Style>,
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
            hover_style: None,
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
    // 键盘与鼠标两个闭包共享同一对回调(Handler 不 Clone,经 Rc<RefCell> 转发)。
    let on_change = Rc::new(RefCell::new(props.on_change.take()));
    let on_select = Rc::new(RefCell::new(props.on_select.take()));
    // 悬停行(仅视觉;光标/勾选状态不受影响)。
    let mut hover = hooks.use_state(|| None::<usize>);
    if !active || item_count == 0 {
        *hover.write_no_update() = None;
    }

    hooks.use_event_handler(EventScope::Current, EventPriority::Normal, {
        let on_change = Rc::clone(&on_change);
        let on_select = Rc::clone(&on_select);
        move |event| {
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
                        toggle(&selected, index);
                        let changed_items = selected_items(&items, &selected.read());
                        (on_change.borrow_mut())(changed_items);
                    }
                    EventResult::Consumed
                }
                KeyCode::Enter => {
                    let chosen_items = selected_items(&items, &selected.read());
                    (on_select.borrow_mut())(chosen_items);
                    EventResult::Consumed
                }
                _ => EventResult::Ignored,
            }
        }
    });

    // 鼠标:悬停高亮该行、滚轮移动光标(j/k 同义)、点击条目 = 移动光标并切换勾选(空格同义)。
    // 行命中按 ListItem 的实际高度计算;未启用鼠标时不注册。
    if pointer_enabled(&hooks) {
        use_pointer_with_layout_sync(
            &mut hooks,
            PointerOptions {
                track_move: true,
                ..Default::default()
            },
            {
                let on_change = Rc::clone(&on_change);
                let items = props.items.clone();
                let heights: Vec<_> = items
                    .iter()
                    .cloned()
                    .map(|item| item.into().height())
                    .collect();
                move |ev| {
                    let hit =
                        item_at_position(ev.area, ev.position, state.read().offset(), &heights);
                    match ev.kind {
                        PointerKind::Enter | PointerKind::Move if active && item_count > 0 => {
                            if hover.get() != hit {
                                hover.set(hit);
                                EventResult::Consumed
                            } else {
                                EventResult::Ignored
                            }
                        }
                        PointerKind::Leave => {
                            if hover.get().is_some() {
                                hover.set(None);
                            }
                            EventResult::Ignored
                        }
                        PointerKind::ScrollUp if active && item_count > 0 => {
                            state.write().select_previous();
                            EventResult::Consumed
                        }
                        PointerKind::ScrollDown if active && item_count > 0 => {
                            state.write().select_next();
                            EventResult::Consumed
                        }
                        PointerKind::Click { .. } if active && item_count > 0 => {
                            if let Some(index) = hit {
                                state.write().select(Some(index));
                                toggle(&selected, index);
                                let changed_items = selected_items(&items, &selected.read());
                                (on_change.borrow_mut())(changed_items);
                                EventResult::Consumed
                            } else {
                                EventResult::Ignored
                            }
                        }
                        _ => EventResult::Ignored,
                    }
                }
            },
        );
    }

    // 主题解析:每个 slot 铺底,对应 props 的 Option<Style> 在上 patch(None → 用主题)。
    let theme = hooks.use_component_theme::<MultiSelectTheme>();
    let style = resolve_style(theme.style, props.style);
    let border_style = resolve_style(theme.border_style, props.border_style);
    let highlight_style = resolve_style(theme.highlight_style, props.highlight_style);
    let selected_item_style = resolve_style(theme.selected_item_style, props.selected_item_style);
    let hover_style = resolve_style(theme.hover_style, props.hover_style);
    let empty_style = resolve_style(theme.empty_style, props.empty_style);

    let is_empty = props.items.is_empty();
    let selected_snapshot = selected.read().clone();
    // 样式优先级:光标行(List highlight) > 已勾选 > 悬停。
    let hovered = hover
        .get()
        .filter(|index| *index < item_count && Some(*index) != state.read().selected());
    let list_items: Vec<ListItem<'static>> = props
        .items
        .clone()
        .into_iter()
        .enumerate()
        .map(|(index, item)| {
            let item: ListItem<'static> = item.into();
            if selected_snapshot.contains(&index) {
                item.style(selected_item_style)
            } else if Some(index) == hovered {
                item.style(hover_style)
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

/// 切换某行的勾选态(先收值再写,避免读守卫与写交叠)。
fn toggle(selected: &State<HashSet<usize>>, index: usize) {
    let checked = selected.read().contains(&index);
    if checked {
        selected.write().remove(&index);
    } else {
        selected.write().insert(index);
    }
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

#[cfg(test)]
mod mouse_tests {
    use super::*;
    use crate::mouse_test::Mounted;
    use crossterm::event::{MouseButton, MouseEventKind};
    use std::sync::{Arc, Mutex};

    type Log = Arc<Mutex<Vec<String>>>;

    #[derive(Clone, Default, Props)]
    struct HarnessProps {
        log: Log,
    }

    #[component]
    fn Harness(props: &HarnessProps, _hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let log = props.log.clone();
        crate::element!(MultiSelect<String>(
            items: vec!["a".to_string(), "b".to_string(), "c".to_string()],
            default_index: Some(0),
            on_change: move |items: Vec<String>| log.lock().unwrap().push(items.join(",")),
        ))
    }

    // 点击行 = 移动光标并切换勾选;再点一次取消。
    #[test]
    fn clicking_row_toggles_check() {
        let log: Log = Default::default();
        let mut m = Mounted::mount(crate::element!(Harness(log: log.clone())), true);

        m.dispatch(MouseEventKind::Down(MouseButton::Left), 5, 2);
        m.dispatch(MouseEventKind::Up(MouseButton::Left), 5, 2);
        assert_eq!(*log.lock().unwrap(), vec!["b".to_string()]);

        m.dispatch(MouseEventKind::Down(MouseButton::Left), 5, 2);
        m.dispatch(MouseEventKind::Up(MouseButton::Left), 5, 2);
        assert_eq!(*log.lock().unwrap(), vec!["b".to_string(), String::new()]);
    }
    #[test]
    fn clicking_border_does_not_toggle() {
        let log: Log = Default::default();
        let mut m = Mounted::mount(crate::element!(Harness(log: log.clone())), true);
        for (x, y) in [(5, 0), (0, 2), (39, 2), (5, 9)] {
            m.dispatch(MouseEventKind::Down(MouseButton::Left), x, y);
            m.dispatch(MouseEventKind::Up(MouseButton::Left), x, y);
        }
        assert!(log.lock().unwrap().is_empty());
    }
    #[component]
    fn MultilineHarness(props: &HarnessProps, _hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let log = props.log.clone();
        crate::element!(MultiSelect<ListItem<'static>>(
            items: vec![ListItem::new("first\nsecond"), ListItem::new("next")],
            default_index: Some(0),
            on_change: move |items: Vec<ListItem<'static>>| log.lock().unwrap().push(
                items.iter().map(|item| item.height().to_string()).collect::<Vec<_>>().join(",")
            ),
        ))
    }

    #[test]
    fn clicking_second_line_toggles_the_same_item() {
        let log: Log = Default::default();
        let mut m = Mounted::mount(crate::element!(MultilineHarness(log: log.clone())), true);
        m.dispatch(MouseEventKind::Down(MouseButton::Left), 5, 2);
        m.dispatch(MouseEventKind::Up(MouseButton::Left), 5, 2);
        assert_eq!(*log.lock().unwrap(), vec!["2".to_string()]);
        m.dispatch(MouseEventKind::Down(MouseButton::Left), 5, 3);
        m.dispatch(MouseEventKind::Up(MouseButton::Left), 5, 3);
        assert_eq!(
            *log.lock().unwrap(),
            vec!["2".to_string(), "2,1".to_string()]
        );
    }
}
