// VirtualList 组件：基于 `tui-widget-list` 的虚拟列表。

use crate::components::theme::resolve_style;
use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    style::Style,
    widgets::{Block, Widget},
};
use ratatui_kit::{
    Component, ComponentTheme, Handler, Palette, PointerKind, PointerOptions, Props, State,
    UseEffect, UseEventHandler, UseState,
    hooks::use_pointer::{pointer_enabled, use_pointer_with_layout_sync},
    input::{EventPriority, EventResult, EventScope},
    with_layout_style,
};
use std::{cell::RefCell, rc::Rc};
use tui_widget_list::{
    ListBuildContext, ListBuilder, ListState, ListView as TuiListView, ScrollAxis, ScrollDirection,
    hit_test::Hit,
};

/// VirtualList 组件的主题 slot。
///
/// 注:虚拟列表的**选中/悬停态由 `render_item` 闭包按
/// `VirtualItemContext::is_selected`/`is_hovered` 自绘**,不由 widget 统一着色,
/// 故本 slot 只提供列表基础样式。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtualListTheme {
    /// 列表基础样式。
    pub style: Style,
}

impl ComponentTheme for VirtualListTheme {
    fn from_palette(palette: &Palette) -> Self {
        Self {
            style: Style::new().fg(palette.fg),
        }
    }
}

impl Default for VirtualListTheme {
    fn default() -> Self {
        Self::from_palette(&Palette::default())
    }
}

/// `render_item` 的渲染上下文:在 tui-widget-list 的 [`ListBuildContext`] 之上
/// 补充组件侧指针状态。`index`/`is_selected` 等经 `Deref` 原样可用。
pub struct VirtualItemContext<'a> {
    /// 包裹的上游渲染上下文(经 `Deref` 透出 `index`/`is_selected` 等)。
    pub inner: &'a ListBuildContext,
    /// 指针当前悬停在本条目上(行→索引由 `hit_test` 映射;无鼠标或未悬停为 false)。
    pub is_hovered: bool,
}

impl<'a> VirtualItemContext<'a> {
    /// 由上游上下文 + 悬停标志构造(自建列表组件转发 `is_hovered` 时用)。
    pub fn new(inner: &'a ListBuildContext, is_hovered: bool) -> Self {
        Self { inner, is_hovered }
    }
}

impl std::ops::Deref for VirtualItemContext<'_> {
    type Target = ListBuildContext;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

type VirtualItemRenderer<'a, W> = dyn Fn(&VirtualItemContext) -> (W, u16) + Send + Sync + 'a;

// 虚拟列表 item 渲染器。返回值是 `(widget, main_axis_size)`。
pub struct RenderVirtualItem<'a, W>(Option<Box<VirtualItemRenderer<'a, W>>>);

impl<W> RenderVirtualItem<'_, W> {
    fn call(&self, context: &VirtualItemContext) -> (W, u16) {
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
    F: Fn(&VirtualItemContext) -> (W, u16) + Send + Sync + 'static,
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
    /// 是否启用内置键盘/点击/滚轮导航;悬停视觉始终可用。
    pub active: bool,
    pub default_index: Option<usize>,
    pub on_select: Handler<'static, usize>,
    pub scroll_axis: ScrollAxis,
    pub scroll_direction: ScrollDirection,
    // 基础样式覆盖。`None` 用 `VirtualListTheme`,`Some(s)` 以 `theme.patch(s)` 覆盖。
    pub style: Option<Style>,
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
            style: None,
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

// 基于虚拟渲染的列表组件。
pub struct VirtualList<W>
where
    W: Widget + 'static,
{
    state: Option<State<ListState>>,
    item_count: usize,
    render_item: RenderVirtualItem<'static, W>,
    // 悬停条目索引(None = 未悬停);draw 时转发给 render_item 的 is_hovered。
    hover: Option<State<Option<usize>>>,
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
            hover: None,
            scroll_axis: props.scroll_axis,
            scroll_direction: props.scroll_direction,
            // 样式待 update 经主题解析后写入。
            style: Style::default(),
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
            hover: None,
            scroll_axis: props.scroll_axis,
            scroll_direction: props.scroll_direction,
            // 样式待 update 经主题解析后写入。
            style: Style::default(),
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
        // 键盘与鼠标两个闭包共享同一个回调(Handler 不 Clone,经 Rc<RefCell> 转发)。
        let on_select = Rc::new(RefCell::new(props.on_select.take()));
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
                            (on_select.borrow_mut())(index);
                        }
                        EventResult::Consumed
                    }
                    _ => EventResult::Ignored,
                }
            }
        });

        // 鼠标:滚轮移动选中(j/k 同义);点击条目 = 选中并确认(Enter 同义);
        // Enter/Move 跟踪悬停条目,经 `VirtualItemContext::is_hovered` 交给
        // render_item 自绘悬停态。行→索引交给 tui-widget-list 的 hit_test
        //(变高条目/截断首项/边框都算对)。
        let hover = hooks.use_state(|| None::<usize>);
        if pointer_enabled(&hooks) {
            use_pointer_with_layout_sync(
                &mut hooks,
                PointerOptions {
                    // 悬停跟踪需要 Move/Enter/Leave 上报。
                    track_move: true,
                    ..Default::default()
                },
                {
                    let on_select = Rc::clone(&on_select);
                    let mut hover = hover;
                    move |ev| match ev.kind {
                        PointerKind::Enter | PointerKind::Move if item_count > 0 => {
                            // 先把命中收成值再进 match:match scrutinee 的读 guard 会
                            // 存活到块尾,块内写同一 State = parking_lot 死锁(渲染线程冻结)。
                            let hit = state.read().hit_test(ev.position.x, ev.position.y);
                            let next = match hit {
                                Some(Hit::Item(index)) if index < item_count => Some(index),
                                // 内区空白(条目间隙/不满一屏的尾部)不算悬停。
                                _ => None,
                            };
                            if hover.get() != next {
                                hover.set(next);
                                EventResult::Consumed
                            } else {
                                EventResult::Ignored
                            }
                        }
                        PointerKind::Leave => {
                            // 无论 active 与否都要清悬停,避免停用后残留高亮。
                            if hover.get().is_some() {
                                hover.set(None);
                            }
                            EventResult::Ignored
                        }
                        PointerKind::ScrollUp if active && item_count > 0 => {
                            state.write().previous();
                            EventResult::Consumed
                        }
                        PointerKind::ScrollDown if active && item_count > 0 => {
                            state.write().next();
                            EventResult::Consumed
                        }
                        PointerKind::Click { .. } if active && item_count > 0 => {
                            // 同上:命中先收成值,避免 scrutinee 读 guard 死锁。
                            let hit = state.read().hit_test(ev.position.x, ev.position.y);
                            match hit {
                                Some(Hit::Item(index)) if index < item_count => {
                                    state.write().select(Some(index));
                                    (on_select.borrow_mut())(index);
                                    EventResult::Consumed
                                }
                                // 点在内区空白(条目间隙/不满一屏的尾部)不算命中,放行。
                                _ => EventResult::Ignored,
                            }
                        }
                        _ => EventResult::Ignored,
                    }
                },
            );
        }

        updater.set_layout_style(layout_style);

        // 主题解析:theme slot 铺底,props 的 Option<Style> 在上 patch(None → 用主题)。
        // use_component_theme 返回 owned 值、读后即弃守卫,不与 &mut updater 冲突。
        let theme = updater.use_component_theme::<VirtualListTheme>();
        let style = resolve_style(theme.style, props.style);

        *self = Self {
            state: Some(state),
            hover: Some(hover),
            style,
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
        let hover = self.hover;
        let builder = ListBuilder::new(move |context: &ListBuildContext| {
            // 组件知道悬停行(hit_test),经扩展上下文把 is_hovered 传给调用方自绘。
            let is_hovered = hover
                .as_ref()
                .and_then(|hover| hover.get())
                .is_some_and(|index| index == context.index);
            render_item.call(&VirtualItemContext {
                inner: context,
                is_hovered,
            })
        });

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

#[cfg(test)]
mod mouse_tests {
    use super::*;
    use crate::{AnyElement, Hooks, mouse_test::Mounted};
    use crossterm::event::{MouseButton, MouseEventKind};
    use ratatui::style::Modifier;
    use ratatui_kit_macros::{Props, component};
    use std::sync::{Arc, Mutex};

    type Log = Arc<Mutex<Vec<usize>>>;

    #[derive(Clone, Default, Props)]
    struct HarnessProps {
        log: Log,
        active: Option<bool>,
    }

    #[component]
    fn Harness(props: &HarnessProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let log = props.log.clone();
        let state = hooks.use_state(ListState::default);
        crate::element!(VirtualList<ratatui::text::Text<'static>>(
            state: state,
            active: props.active.unwrap_or(true),
            item_count: 5usize,
            render_item: |context: &VirtualItemContext| {
                let text = ratatui::text::Text::from(format!("item {}", context.index));
                let styled = if context.is_hovered {
                    text.style(ratatui::style::Style::new().add_modifier(Modifier::BOLD))
                } else {
                    text
                };
                (styled, 1u16)
            },
            on_select: move |index: usize| log.lock().unwrap().push(index),
        ))
    }

    // 点击条目 = 选中并确认;滚轮移动选中。
    #[test]
    fn clicking_item_selects_and_wheel_moves() {
        let log: Log = Default::default();
        let mut m = Mounted::mount(crate::element!(Harness(log: log.clone())), true);

        m.dispatch(MouseEventKind::Down(MouseButton::Left), 5, 1);
        m.dispatch(MouseEventKind::Up(MouseButton::Left), 5, 1);
        assert_eq!(*log.lock().unwrap(), vec![1]);

        m.dispatch(MouseEventKind::ScrollDown, 5, 1);
        m.dispatch(MouseEventKind::ScrollDown, 5, 1);
        m.dispatch_key(crossterm::event::KeyCode::Enter);
        assert_eq!(*log.lock().unwrap(), vec![1, 3]);
    }

    // 悬停经 VirtualItemContext::is_hovered 传给 render_item 自绘(选中行除外:
    // 本 harness 无选中样式,悬停行应为 BOLD,移开后消失)。
    #[test]
    fn hovering_item_is_hovered_in_render_item() {
        let log: Log = Default::default();
        let mut m = Mounted::mount(crate::element!(Harness(log: log.clone())), true);

        m.dispatch(MouseEventKind::Moved, 5, 2);
        let buf = m.buffer();
        assert!(
            buf[(1, 2)].style().add_modifier.contains(Modifier::BOLD),
            "悬停行应为 BOLD"
        );
        assert!(
            !buf[(1, 1)].style().add_modifier.contains(Modifier::BOLD),
            "未悬停行不受影响"
        );

        m.dispatch(MouseEventKind::Moved, 39, 9);
        let buf = m.buffer();
        assert!(
            !buf[(1, 2)].style().add_modifier.contains(Modifier::BOLD),
            "移开后高亮消失"
        );
    }
    #[test]
    fn custom_navigation_keeps_hover_without_builtin_actions() {
        let log: Log = Default::default();
        let mut m = Mounted::mount(
            crate::element!(Harness(log: log.clone(), active: Some(false))),
            true,
        );
        m.dispatch(MouseEventKind::Moved, 5, 2);
        assert!(
            m.buffer()[(5, 2)]
                .style()
                .add_modifier
                .contains(Modifier::BOLD)
        );
        m.dispatch(MouseEventKind::Down(MouseButton::Left), 5, 2);
        m.dispatch(MouseEventKind::Up(MouseButton::Left), 5, 2);
        assert!(log.lock().unwrap().is_empty());
    }
    #[test]
    fn movement_within_hovered_item_does_not_request_redraw() {
        let mut m = Mounted::mount(crate::element!(Harness), true);
        assert!(m.dispatch(MouseEventKind::Moved, 5, 2));
        assert!(!m.dispatch(MouseEventKind::Moved, 6, 2));
    }
}
