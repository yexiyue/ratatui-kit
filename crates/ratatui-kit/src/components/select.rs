// Select 组件：带键盘事件处理的单选列表。

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
use std::{cell::RefCell, rc::Rc};

/// Select 组件的主题 slot。高亮为「`on_accent` 前景 + `selection` 底」的配对。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectTheme {
    /// 列表项常规样式。
    pub style: Style,
    /// 边框样式。
    pub border_style: Style,
    /// 选中项高亮样式。
    pub highlight_style: Style,
    /// 悬停行样式(指针位于该行上;选中行仍用 `highlight_style`)。
    pub hover_style: Style,
    /// 空态提示样式。
    pub empty_style: Style,
}

impl ComponentTheme for SelectTheme {
    fn from_palette(palette: &Palette) -> Self {
        Self {
            style: Style::new().fg(palette.fg),
            border_style: Style::new().fg(palette.border),
            highlight_style: Style::new().fg(palette.on_accent).bg(palette.selection),
            hover_style: Style::new()
                .bg(palette.surface)
                .fg(palette.fg)
                .add_modifier(Modifier::BOLD),
            empty_style: Style::new().fg(palette.warning),
        }
    }
}

impl Default for SelectTheme {
    fn default() -> Self {
        Self::from_palette(&Palette::default())
    }
}

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
    // 以下样式覆盖:`None` 用 `SelectTheme`,`Some(s)` 以 `theme.patch(s)` 覆盖。
    pub style: Option<Style>,
    pub border_style: Option<Style>,
    pub highlight_style: Option<Style>,
    pub hover_style: Option<Style>,
    pub empty_style: Option<Style>,
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
            style: None,
            border_style: None,
            highlight_style: None,
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
    // 键盘与鼠标两个闭包共享同一个回调(Handler 不 Clone,经 Rc<RefCell> 转发)。
    let on_select = Rc::new(RefCell::new(props.on_select.take()));
    // 悬停行(仅视觉;不入 ListState,不影响键盘选中)。
    let mut hover = hooks.use_state(|| None::<usize>);
    if !active || item_count == 0 {
        *hover.write_no_update() = None;
    }

    hooks.use_event_handler(EventScope::Current, EventPriority::Normal, {
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
                KeyCode::Enter => {
                    let selected_index = state.read().selected();
                    if let Some(index) = selected_index
                        && let Some(item) = items.get(index)
                    {
                        (on_select.borrow_mut())(item.clone());
                    }
                    EventResult::Consumed
                }
                _ => EventResult::Ignored,
            }
        }
    });

    // 鼠标:悬停高亮该行、滚轮移动选中(j/k 同义)、点击条目 = 选中并确认(Enter 同义)。
    // 点击与悬停共用实际条目高度映射,边框和尾部空白不属于条目。
    // 未启用鼠标捕获的应用不注册(use_pointer 会 debug 断言;分支进程内稳定)。
    if pointer_enabled(&hooks) {
        use_pointer_with_layout_sync(
            &mut hooks,
            PointerOptions {
                track_move: true,
                ..Default::default()
            },
            {
                let on_select = Rc::clone(&on_select);
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
                            if let Some(index) = hit
                                && let Some(item) = items.get(index)
                            {
                                state.write().select(Some(index));
                                (on_select.borrow_mut())(item.clone());
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
    let theme = hooks.use_component_theme::<SelectTheme>();
    let style = resolve_style(theme.style, props.style);
    let border_style = resolve_style(theme.border_style, props.border_style);
    let highlight_style = resolve_style(theme.highlight_style, props.highlight_style);
    let hover_style = resolve_style(theme.hover_style, props.hover_style);
    let empty_style = resolve_style(theme.empty_style, props.empty_style);

    let is_empty = props.items.is_empty();
    // 悬停行打上 hover 样式(选中行除外,它由 List 的 highlight_style 负责)。
    let hovered = hover
        .get()
        .filter(|index| *index < item_count && Some(*index) != state.read().selected());
    let list_items: Vec<ListItem<'static>> = props
        .items
        .clone()
        .into_iter()
        .enumerate()
        .map(|(index, item)| {
            let item = item.into();
            if Some(index) == hovered {
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

#[cfg(test)]
mod mouse_tests {
    use super::*;
    use crate::mouse_test::Mounted;
    use crossterm::event::{KeyCode, MouseButton, MouseEventKind};
    use ratatui::style::Modifier;
    use std::{
        cell::RefCell,
        rc::Rc,
        sync::{Arc, Mutex},
    };

    type Log = Arc<Mutex<Vec<String>>>;
    type StateSlot = Rc<RefCell<Option<State<ListState>>>>;

    #[derive(Clone, Default, Props)]
    struct HarnessProps {
        log: Log,
        slot: StateSlot,
    }

    #[component]
    fn Harness(props: &HarnessProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let state = hooks.use_state(ListState::default);
        if props.slot.borrow_mut().is_none() {
            *props.slot.borrow_mut() = Some(state);
        }
        let log = props.log.clone();
        crate::element!(Select<String>(
            items: vec!["a".to_string(), "b".to_string(), "c".to_string()],
            default_index: Some(0),
            state: state,
            on_select: move |item: String| log.lock().unwrap().push(item),
        ))
    }

    fn mount(slot: &StateSlot) -> Mounted {
        let m = Mounted::mount(
            crate::element!(Harness(log: Log::default(), slot: slot.clone())),
            true,
        );
        assert!(slot.borrow().is_some());
        m
    }

    fn selected(slot: &StateSlot) -> Option<usize> {
        slot.borrow().unwrap().read().selected()
    }

    // 点击行 = 选中并确认(Enter 同义)。
    #[test]
    fn clicking_row_selects_and_confirms() {
        let slot: StateSlot = Default::default();
        let mut m = mount(&slot);

        m.dispatch(MouseEventKind::Down(MouseButton::Left), 5, 2);
        m.dispatch(MouseEventKind::Up(MouseButton::Left), 5, 2);
        assert_eq!(selected(&slot), Some(1));
    }

    #[test]
    fn clicking_border_does_not_confirm() {
        let slot: StateSlot = Default::default();
        let log: Log = Default::default();
        let mut m = Mounted::mount(
            crate::element!(Harness(log: log.clone(), slot: slot.clone())),
            true,
        );
        for (x, y) in [(5, 0), (0, 2), (39, 2), (5, 9)] {
            m.dispatch(MouseEventKind::Down(MouseButton::Left), x, y);
            m.dispatch(MouseEventKind::Up(MouseButton::Left), x, y);
        }
        assert!(log.lock().unwrap().is_empty());
        assert_eq!(selected(&slot), Some(0));
    }

    #[component]
    fn MultilineHarness(props: &HarnessProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let state = hooks.use_state(ListState::default);
        *props.slot.borrow_mut() = Some(state);
        crate::element!(Select<ListItem<'static>>(
            items: vec![ListItem::new("first\nsecond"), ListItem::new("next")],
            state: state,
            default_index: Some(0),
        ))
    }

    #[test]
    fn clicking_second_line_selects_the_same_item() {
        let slot: StateSlot = Default::default();
        let mut m = Mounted::mount(crate::element!(MultilineHarness(slot: slot.clone())), true);
        m.dispatch(MouseEventKind::Down(MouseButton::Left), 5, 2);
        m.dispatch(MouseEventKind::Up(MouseButton::Left), 5, 2);
        assert_eq!(selected(&slot), Some(0));
        m.dispatch(MouseEventKind::Down(MouseButton::Left), 5, 3);
        m.dispatch(MouseEventKind::Up(MouseButton::Left), 5, 3);
        assert_eq!(selected(&slot), Some(1));
    }

    // 滚轮移动选中(j/k 同义)。
    #[test]
    fn wheel_moves_selection() {
        let slot: StateSlot = Default::default();
        let mut m = mount(&slot);

        m.dispatch(MouseEventKind::ScrollDown, 5, 2);
        assert_eq!(selected(&slot), Some(1));
        m.dispatch(MouseEventKind::ScrollUp, 5, 2);
        assert_eq!(selected(&slot), Some(0));
    }

    // 悬停高亮该行(选中行除外),移开后恢复。默认 hover 样式带 BOLD
    //(默认 palette 的 surface 是 Reset,单看底色不可见)。
    #[test]
    fn hovering_row_highlights_it() {
        let slot: StateSlot = Default::default();
        let mut m = mount(&slot);

        m.dispatch(MouseEventKind::Moved, 5, 2);
        let buf = m.buffer();
        assert!(
            buf[(1, 2)].style().add_modifier.contains(Modifier::BOLD),
            "悬停行应为 BOLD"
        );
        assert!(
            !buf[(1, 1)].style().add_modifier.contains(Modifier::BOLD),
            "选中行(第 0 项)保持 highlight 而非 hover"
        );

        m.dispatch(MouseEventKind::Moved, 39, 9);
        let buf = m.buffer();
        assert!(
            !buf[(1, 2)].style().add_modifier.contains(Modifier::BOLD),
            "移开后高亮消失"
        );
    }

    // 键盘语义不受鼠标接线影响。
    #[test]
    fn keyboard_still_works() {
        let slot: StateSlot = Default::default();
        let mut m = mount(&slot);

        m.dispatch_key(KeyCode::Char('j'));
        assert_eq!(selected(&slot), Some(1));
        m.dispatch_key(KeyCode::Enter);
        m.dispatch_key(KeyCode::Enter);
        let _ = m;
    }
}
