use crate::components::scroll_view::{
    AxisGeometry, ScrollbarGeometry, ScrollbarInteraction, ScrollbarStyle,
};
use crate::components::theme::resolve_style;
use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Position, Rect},
    style::{Modifier, Style},
    widgets::{Block, ScrollbarOrientation, ScrollbarState},
};
use ratatui_kit::{
    Component, ComponentTheme, Handler, Palette, PointerKind, PointerOptions, Props, State,
    UseEffect, UseEventHandler, UsePointer, UseState,
    hooks::use_pointer::{pointer_enabled, use_pointer_with_layout_sync},
    input::{EventPriority, EventResult, EventScope},
    with_layout_style,
};
use std::{cell::RefCell, hash::Hash, rc::Rc};
use tui_tree_widget::{TreeItem, TreeState};

/// TreeSelect 组件的主题 slot。默认提供可见选中态(`on_accent` 前景 + `selection` 底)。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeSelectTheme {
    /// 树基础样式。
    pub style: Style,
    /// 选中项高亮样式。
    pub highlight_style: Style,
    /// 悬停行样式(指针位于该行上;选中行仍用 `highlight_style`)。
    pub hover_style: Style,
}

impl ComponentTheme for TreeSelectTheme {
    fn from_palette(palette: &Palette) -> Self {
        Self {
            style: Style::new().fg(palette.fg),
            highlight_style: Style::new().fg(palette.on_accent).bg(palette.selection),
            // 默认 palette 的 surface 是 Reset,单靠底色不可见,补 BOLD(与 Select 一致)。
            hover_style: Style::new()
                .bg(palette.surface)
                .fg(palette.fg)
                .add_modifier(Modifier::BOLD),
        }
    }
}

impl Default for TreeSelectTheme {
    fn default() -> Self {
        Self::from_palette(&Palette::default())
    }
}

// 树形组件的属性定义
#[with_layout_style(margin, offset, width, height)]
#[derive(Props)]
pub struct TreeSelectProps<T>
where
    T: Sync + Send + Clone + Eq + Hash + 'static,
{
    // 树形组件的状态，可选
    pub state: Option<State<TreeState<T>>>,

    // 树形节点项列表
    pub items: Vec<TreeItem<'static, T>>,
    /// 是否启用内置键盘/点击/滚轮导航;悬停视觉始终可用。默认关闭。
    pub active: bool,
    // 默认选中的节点路径；例如 `["components", "input"]`。
    pub default_selection: Vec<T>,
    // 当前项确认选择时触发。
    pub on_select: Handler<'static, T>,

    /// 可拖拽滚动条外观;None 隐藏。拖拽独立于 active,不改变选中项。
    pub scrollbar: Option<ScrollbarStyle<'static>>,
    // 基础样式覆盖。`None` 用 `TreeSelectTheme`,`Some(s)` 以 `theme.patch(s)` 覆盖。
    pub style: Option<Style>,

    // 选中项样式覆盖。`None` 用 `TreeSelectTheme`(默认可见),`Some(s)` 以 `theme.patch(s)` 覆盖。
    pub highlight_style: Option<Style>,
    // 悬停行样式覆盖。`None` 用 `TreeSelectTheme`,`Some(s)` 以 `theme.patch(s)` 覆盖。
    pub hover_style: Option<Style>,
    // 显示在选中项前面的符号（会将所有项右移）
    pub highlight_symbol: &'static str,

    // 显示在已关闭节点前面的符号（子节点当前不可见）
    pub node_closed_symbol: &'static str,
    // 显示在已打开节点前面的符号（子节点当前可见）
    pub node_open_symbol: &'static str,
    // 显示在没有子节点的节点前面的符号
    pub node_no_children_symbol: &'static str,
    // 可选的边框块。
    pub block: Option<Block<'static>>,
}

impl<T> Default for TreeSelectProps<T>
where
    T: Sync + Send + Clone + Eq + Hash,
{
    fn default() -> Self {
        Self {
            state: None,
            items: vec![],
            active: false,
            default_selection: Vec::new(),
            on_select: Handler::default(),
            scrollbar: None,
            style: None,
            highlight_style: None,
            hover_style: None,
            highlight_symbol: "",
            node_closed_symbol: "\u{25b6} ", // 向右箭头
            node_open_symbol: "\u{25bc} ",   // 向下箭头
            node_no_children_symbol: "  ",
            block: None,
            margin: Default::default(),
            offset: Default::default(),
            width: Default::default(),
            height: Default::default(),
        }
    }
}

/// 滚轮一格滚动的行数,与常见终端编辑器的默认步长一致。
const WHEEL_LINES: usize = 3;

/// 按 identifier 递归查找树节点(点击命中叶子判定用)。
fn find_item<'a, T>(items: &'a [TreeItem<'static, T>], id: &T) -> Option<&'a TreeItem<'static, T>>
where
    T: Clone + Eq + Hash,
{
    for item in items {
        if item.identifier() == id {
            return Some(item);
        }
        if let Some(found) = find_item(item.children(), id) {
            return Some(found);
        }
    }
    None
}

/// 每个可见条目的 `(起始行 y, 行高, identifier)`,y 相对视口顶、已按
/// `viewport_height` 限制底边。由上游公开 API 精确折叠:
/// `flatten`(展开态)+ `TreeItem::height()`(多行 Text 条目)+ `get_offset()`
/// (展开后的条目索引)。上游只绘制完整放入视口的条目,不截断多行节点。
///
/// 注意**不要**用逐行扫 `rendered_at` 代替:它对内容下方任意 y 都返回最后一个
/// 渲染条目,末条目行高会被虚增到视口底。
fn visible_rows<T>(
    state: &TreeState<T>,
    items: &[TreeItem<'static, T>],
    viewport_height: u16,
) -> Vec<(u16, u16, Vec<T>)>
where
    T: Clone + Eq + Hash,
{
    let mut rows = Vec::new();
    let mut y: u16 = 0;
    for flat in state.flatten(items).into_iter().skip(state.get_offset()) {
        let height = flat.item.height();
        if height > usize::from(viewport_height.saturating_sub(y)) {
            break;
        }
        rows.push((y, height as u16, flat.identifier));
        y += height as u16;
    }
    rows
}

/// 命中树节点的实际可见行;`area` 应传树的内容区(有边框时传 `block.inner(area)`)。
/// 视口尾部空白返回 `None`,与上游 `TreeState::rendered_at` 的最近行语义不同。
pub fn tree_item_at_position<T>(
    state: &TreeState<T>,
    items: &[TreeItem<'static, T>],
    area: Rect,
    position: Position,
) -> Option<Vec<T>>
where
    T: Clone + Eq + Hash,
{
    if !area.contains(position) {
        return None;
    }
    let row = position.y - area.y;
    visible_rows(state, items, area.height)
        .into_iter()
        .find(|(start, height, _)| row >= *start && row < start.saturating_add(*height))
        .map(|(_, _, path)| path)
}

fn sync_default_tree_selection<T>(
    state: &mut TreeState<T>,
    last_default_selection: &mut Option<Vec<T>>,
    default_selection: &[T],
) where
    T: Clone + Eq + Hash,
{
    let default_changed = last_default_selection.as_deref() != Some(default_selection);

    if default_changed {
        let first_sync = last_default_selection.is_none();
        *last_default_selection = Some(default_selection.to_vec());
        // 首次同步且 `default_selection` 为空 —— 即调用方**根本没传**这个 prop(它默认为空 Vec)。
        // 此时不能 `select(vec![])`:空路径在 tui-tree-widget 里表示**清除选中**,会把调用方
        // 经 `state` 自行预设的选中抹掉。组件树是父先子后更新,这次清除总发生在父组件 seeding
        // 之后,表现为「传了 state、预设了选中,挂载后却没有任何高亮」。
        // 「从非空显式改回空 = 清除选中」仍受支持(那时 first_sync 为 false)。
        if first_sync && default_selection.is_empty() {
            return;
        }
        state.select(default_selection.to_vec());
        open_ancestors(state, default_selection);
    } else if state.selected().is_empty() && !default_selection.is_empty() {
        state.select(default_selection.to_vec());
        open_ancestors(state, default_selection);
    }
}

fn open_ancestors<T>(state: &mut TreeState<T>, selection: &[T])
where
    T: Clone + Eq + Hash,
{
    for end in 1..selection.len() {
        state.open(selection[..end].to_vec());
    }
}

// 树形组件实现。
pub struct TreeSelect<T>
where
    T: Sync + Send + Clone + Eq + Hash + 'static,
{
    state: Option<State<TreeState<T>>>,
    items: Vec<TreeItem<'static, T>>,
    scrollbar: Option<ScrollbarStyle<'static>>,
    style: Style,
    highlight_style: Style,
    hover_style: Style,
    // 悬停节点路径;同一节点内移动不写状态/重绘。
    hover: Option<State<Option<Vec<T>>>>,
    highlight_symbol: &'static str,
    node_closed_symbol: &'static str,
    node_open_symbol: &'static str,
    node_no_children_symbol: &'static str,
    block: Option<Block<'static>>,
    tree_is_valid: bool,
    scrollbar_interaction: Option<State<ScrollbarInteraction>>,
}

impl<T> TreeSelect<T>
where
    T: Sync + Send + Clone + Eq + Hash + 'static,
{
    fn from_props(props: &TreeSelectProps<T>) -> Self {
        Self {
            scrollbar_interaction: None,
            state: props.state,
            items: props.items.clone(),
            scrollbar: props.scrollbar.clone(),
            // 样式待 update 经主题解析后写入。
            style: Style::default(),
            highlight_style: Style::default(),
            hover_style: Style::default(),
            hover: None,
            highlight_symbol: props.highlight_symbol,
            node_closed_symbol: props.node_closed_symbol,
            node_open_symbol: props.node_open_symbol,
            node_no_children_symbol: props.node_no_children_symbol,
            block: props.block.clone(),
            tree_is_valid: tui_tree_widget::Tree::new(&props.items).is_ok(),
        }
    }
}

impl<T> Component for TreeSelect<T>
where
    T: Sync + Send + Clone + Eq + Hash + Unpin + 'static,
{
    type Props<'a>
        = TreeSelectProps<T>
    where
        Self: 'a;

    // 根据属性创建新的树形组件实例
    fn new(props: &Self::Props<'_>) -> Self {
        Self::from_props(props)
    }

    fn update(
        &mut self,
        props: &mut Self::Props<'_>,
        mut hooks: crate::Hooks,
        updater: &mut crate::ComponentUpdater,
    ) {
        let layout_style = props.layout_style();
        let mut hooks = hooks.with_context_stack(updater.component_context_stack());
        let local_state = hooks.use_state(TreeState::default);
        let state = props.state.unwrap_or(local_state);

        let default_selection = props.default_selection.clone();
        let mut last_default_selection = hooks.use_state(|| None::<Vec<T>>);
        hooks.use_effect(
            move || {
                let mut last_default = last_default_selection.read().clone();
                sync_default_tree_selection(
                    &mut state.write(),
                    &mut last_default,
                    &default_selection,
                );
                last_default_selection.set(last_default);
            },
            props.default_selection.clone(),
        );

        let active = props.active;
        let has_items = !props.items.is_empty();
        // 键盘与鼠标两个闭包共享同一个回调(Handler 不 Clone,经 Rc<RefCell> 转发)。
        let on_select = Rc::new(RefCell::new(props.on_select.take()));
        hooks.use_event_handler(EventScope::Current, EventPriority::Normal, {
            let on_select = Rc::clone(&on_select);
            move |event| {
                if !active || !has_items {
                    return EventResult::Ignored;
                }

                let Event::Key(key) = event else {
                    return EventResult::Ignored;
                };
                if key.kind != KeyEventKind::Press {
                    return EventResult::Ignored;
                }

                match key.code {
                    KeyCode::Char('h') | KeyCode::Left => {
                        state.write().key_left();
                        EventResult::Consumed
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        state.write().key_down();
                        EventResult::Consumed
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        state.write().key_up();
                        EventResult::Consumed
                    }
                    KeyCode::Char('l') | KeyCode::Right => {
                        state.write().key_right();
                        EventResult::Consumed
                    }
                    KeyCode::Char(' ') => {
                        state.write().toggle_selected();
                        EventResult::Consumed
                    }
                    KeyCode::Enter => {
                        let selected = state.read().selected().last().cloned();
                        if let Some(selected) = selected {
                            (on_select.borrow_mut())(selected);
                        }
                        EventResult::Consumed
                    }
                    _ => EventResult::Ignored,
                }
            }
        });

        // 鼠标:滚轮滚动视口;点击叶子 = 选中并确认(Enter 同义),点击分支 = 选中/
        // 切换展开(点已选中的分支即收起)。行→节点由 `TreeState::rendered_at` 按
        // 上一帧渲染记录映射;Enter/Move 记录悬停位置,draw 时给整行打 hover 补丁。
        let hover = hooks.use_state(|| None::<Vec<T>>);
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
                    let items = props.items.clone();
                    let block = props.block.clone();
                    let mut hover = hover;
                    move |ev| match ev.kind {
                        PointerKind::Enter | PointerKind::Move if has_items => {
                            let inner =
                                block.as_ref().map_or(ev.area, |block| block.inner(ev.area));
                            let next =
                                tree_item_at_position(&state.read(), &items, inner, ev.position);
                            let changed = *hover.read() != next;
                            if changed {
                                hover.set(next);
                                EventResult::Consumed
                            } else {
                                EventResult::Ignored
                            }
                        }
                        PointerKind::Leave => {
                            // 无论 active 与否都要清悬停,避免停用后残留高亮。
                            if hover.read().is_some() {
                                hover.set(None);
                            }
                            EventResult::Ignored
                        }
                        PointerKind::ScrollUp if active && has_items => {
                            state.write().scroll_up(WHEEL_LINES);
                            EventResult::Consumed
                        }
                        PointerKind::ScrollDown if active && has_items => {
                            state.write().scroll_down(WHEEL_LINES);
                            EventResult::Consumed
                        }
                        PointerKind::Click { .. } if active && has_items => {
                            let inner =
                                block.as_ref().map_or(ev.area, |block| block.inner(ev.area));
                            let hit =
                                tree_item_at_position(&state.read(), &items, inner, ev.position)
                                    .and_then(|path| path.last().cloned());
                            match hit {
                                Some(id) => {
                                    // 叶子(无子节点)点击即确认;分支用 click_at 的
                                    // 「选中/再点切换展开」语义。
                                    let is_leaf = find_item(&items, &id)
                                        .is_some_and(|item| item.children().is_empty());
                                    state.write().click_at(ev.position);
                                    if is_leaf {
                                        (on_select.borrow_mut())(id);
                                    }
                                    EventResult::Consumed
                                }
                                None => EventResult::Ignored,
                            }
                        }
                        _ => EventResult::Ignored,
                    }
                },
            );
        }

        let scrollbar_interaction = hooks.use_state(ScrollbarInteraction::default);
        if pointer_enabled(&hooks) {
            hooks.use_pointer(
                PointerOptions {
                    track_move: true,
                    ..Default::default()
                },
                move |event| {
                    scrollbar_interaction
                        .write_no_update()
                        .handle(event, |target, _| {
                            let mut tree = state.write();
                            let current = tree.get_offset();
                            if target > current {
                                tree.scroll_down(target - current);
                            } else {
                                tree.scroll_up(current - target);
                            }
                        })
                },
            );
        }
        updater.set_layout_style(layout_style);

        // 主题解析:theme slot 铺底,props 的 Option<Style> 在上 patch(None → 用主题)。
        // use_component_theme 返回 owned 值、读后即弃守卫,不与 &mut updater 冲突。
        let theme = updater.use_component_theme::<TreeSelectTheme>();
        let style = resolve_style(theme.style, props.style);
        let highlight_style = resolve_style(theme.highlight_style, props.highlight_style);
        let hover_style = resolve_style(theme.hover_style, props.hover_style);

        *self = Self {
            state: Some(state),
            style,
            highlight_style,
            hover_style,
            hover: Some(hover),
            scrollbar_interaction: Some(scrollbar_interaction),
            ..Self::from_props(props)
        };
    }

    // 绘制树形组件
    fn draw(&mut self, drawer: &mut ratatui_kit::ComponentDrawer<'_, '_>) {
        if !self.tree_is_valid {
            if let Some(block) = self.block.as_ref() {
                drawer.render_widget(block, drawer.area);
            }
            return;
        }

        let Ok(mut tree) = tui_tree_widget::Tree::new(&self.items) else {
            return;
        };

        tree = tree
            .style(self.style)
            .highlight_style(self.highlight_style)
            .highlight_symbol(self.highlight_symbol)
            .node_closed_symbol(self.node_closed_symbol)
            .node_open_symbol(self.node_open_symbol)
            .node_no_children_symbol(self.node_no_children_symbol);

        if let Some(block) = self.block.as_ref() {
            tree = tree.block(block.clone());
        }

        // 树实际渲染区 = block 内区(无 block 即组件 area),悬停补丁按同一区域计算。
        let inner = self
            .block
            .as_ref()
            .map_or(drawer.area, |block| block.inner(drawer.area));

        if let Some(state) = &mut self.state {
            // 渲染有状态的树形组件
            drawer.render_stateful_widget(tree, drawer.area, &mut state.write_no_update());

            // 悬停行补丁:tui-tree-widget 没有逐行样式 API,渲染完按「flatten +
            // height + offset」折叠出的精确行表,给指针所在条目的整行(含多行
            // 条目)打 hover_style;选中行除外(它由 highlight_style 负责)。
            if let Some(hovered) = self.hover.as_ref().and_then(|hover| hover.read().clone()) {
                let snapshot = state.read();
                let selected = snapshot.selected().to_vec();
                for (row_y, height, identifier) in
                    visible_rows(&snapshot, &self.items, inner.height)
                {
                    let start = inner.y + row_y;
                    if identifier != hovered {
                        continue;
                    }
                    if identifier != selected {
                        let rect = Rect::new(inner.x, start, inner.width, height);
                        drawer.buffer_mut().set_style(rect, self.hover_style);
                    }
                    break;
                }
            }
            if let Some(interaction) = self.scrollbar_interaction {
                let mut interaction = interaction.write_no_update();
                interaction.geometry = ScrollbarGeometry::default();
                if let Some(appearance) = &self.scrollbar {
                    let snapshot = state.read();
                    let entries = snapshot.flatten(&self.items);
                    let mut remaining = usize::from(inner.height);
                    let mut viewport = 0;
                    for entry in entries.iter().rev() {
                        let height = entry.item.height();
                        if height > remaining {
                            break;
                        }
                        remaining -= height;
                        viewport += 1;
                    }
                    let scrollable = entries.len().saturating_sub(viewport);
                    let track = Rect::new(
                        drawer.area.right().saturating_sub(1),
                        inner.y,
                        1,
                        inner.height,
                    );
                    if let Some(axis) = AxisGeometry::new(
                        track,
                        viewport,
                        scrollable,
                        snapshot.get_offset().min(scrollable),
                        true,
                        appearance.arrow_lengths(),
                    ) {
                        let mut scrollbar_state = ScrollbarState::new(scrollable + 1)
                            .position(axis.offset)
                            .viewport_content_length(viewport);
                        drawer.render_stateful_widget(
                            appearance.widget_with_state(
                                ScrollbarOrientation::VerticalRight,
                                interaction.is_hovered(true),
                                interaction.is_dragging(true),
                            ),
                            track,
                            &mut scrollbar_state,
                        );
                        interaction.geometry.vertical = Some(axis);
                    }
                }
            }
        } else {
            // 渲染无状态的树形组件
            drawer.render_widget(tree, drawer.area);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // visible_rows 是悬停补丁的根基:必须精确处理多行条目与滚动截断。
    #[test]
    fn visible_rows_fold_heights_and_offset() {
        let items = vec![
            TreeItem::new_leaf("a", "single a"),
            TreeItem::new_leaf("m", "multi\nline\nitem"), // 3 行
            TreeItem::new_leaf("z", "single z"),
        ];
        let mut state = TreeState::<&'static str>::default();

        let rows = visible_rows(&state, &items, 5);
        let shape: Vec<(u16, u16)> = rows.iter().map(|(y, h, _)| (*y, *h)).collect();
        assert_eq!(shape, vec![(0, 1), (1, 3), (4, 1)]);
        assert_eq!(rows[1].2, vec!["m"]);

        // scroll_down 钳制依赖渲染记录的 last_biggest_index,先渲染一帧再滚。
        use ratatui::{buffer::Buffer, layout::Rect, widgets::StatefulWidget};
        let area = Rect::new(0, 0, 30, 5);
        let mut buf = Buffer::empty(area);
        tui_tree_widget::Tree::new(&items)
            .unwrap()
            .render(area, &mut buf, &mut state);

        // offset 是展开后的条目索引,滚动 2 项后只剩 z。
        assert!(state.scroll_down(2));
        let rows = visible_rows(&state, &items, 5);
        let shape: Vec<(u16, u16, &str)> = rows.iter().map(|(y, h, id)| (*y, *h, id[0])).collect();
        assert_eq!(shape, vec![(0, 1, "z")]);
    }

    // 多行条目超过视口余量时不绘制,与上游一致。
    #[test]
    fn visible_rows_clip_to_viewport_height() {
        let items = vec![TreeItem::new_leaf("m", "multi\nline\nitem")];
        let state = TreeState::<&'static str>::default();
        let rows = visible_rows(&state, &items, 2);
        assert!(rows.is_empty());
    }

    #[test]
    fn precise_tree_hit_excludes_empty_rows_and_respects_multiline_nodes() {
        let items = vec![
            TreeItem::new_leaf("m", "multi\nline"),
            TreeItem::new_leaf("z", "last"),
        ];
        let state = TreeState::<&str>::default();
        let area = Rect::new(5, 10, 8, 6);
        assert_eq!(
            tree_item_at_position(&state, &items, area, Position::new(5, 11)),
            Some(vec!["m"])
        );
        assert_eq!(
            tree_item_at_position(&state, &items, area, Position::new(5, 12)),
            Some(vec!["z"])
        );
        assert_eq!(
            tree_item_at_position(&state, &items, area, Position::new(5, 13)),
            None
        );
        assert_eq!(
            tree_item_at_position(&state, &items, area, Position::new(4, 10)),
            None
        );
    }

    #[test]
    fn default_selection_opens_ancestors() {
        let mut state = TreeState::<&'static str>::default();
        let mut last_default = None;

        sync_default_tree_selection(&mut state, &mut last_default, &["components", "select"]);

        assert!(state.opened().contains(&vec!["components"]));
    }

    #[test]
    fn default_selection_sets_selected_path() {
        let mut state = TreeState::<&'static str>::default();
        let mut last_default = None;

        sync_default_tree_selection(&mut state, &mut last_default, &["components", "select"]);

        assert_eq!(state.selected(), ["components", "select"]);
    }

    #[test]
    fn absent_default_selection_preserves_seeded_selection() {
        // 调用方传了自己的 `state` 并预设选中,但没传 `default_selection`(默认空)。
        // 首次同步不得把它清掉 —— 否则任何「state 预设选中」的用法都会在挂载帧失效。
        let mut state = TreeState::<&'static str>::default();
        state.select(vec!["components", "select"]);
        let mut last_default = None;

        sync_default_tree_selection(&mut state, &mut last_default, &[]);

        assert_eq!(state.selected(), ["components", "select"]);
    }

    #[test]
    fn empty_default_selection_clears_selected_path() {
        let mut state = TreeState::<&'static str>::default();
        state.select(vec!["components", "select"]);
        let mut last_default = Some(vec!["components", "select"]);

        sync_default_tree_selection(&mut state, &mut last_default, &[]);

        assert!(state.selected().is_empty());
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

    type Log = Arc<Mutex<Vec<String>>>;

    #[derive(Clone, Default, Props)]
    struct HarnessProps {
        log: Log,
        active: Option<bool>,
    }

    fn items() -> Vec<TreeItem<'static, String>> {
        vec![
            TreeItem::new_leaf("one".to_string(), "leaf one"),
            TreeItem::new(
                "dir".to_string(),
                "branch",
                vec![TreeItem::new_leaf("two".to_string(), "leaf two")],
            )
            .unwrap(),
        ]
    }

    #[component]
    fn Harness(props: &HarnessProps, _hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let log = props.log.clone();
        crate::element!(TreeSelect<String>(
            items: items(),
            active: props.active.unwrap_or(true),
            on_select: move |id: String| log.lock().unwrap().push(id),
        ))
    }

    // 点击叶子 = 选中并确认(Enter 同义);点击分支 = 选中/再点展开。
    #[test]
    fn clicking_leaf_confirms_and_branch_toggles() {
        let log: Log = Default::default();
        let mut m = Mounted::mount(crate::element!(Harness(log: log.clone())), true);

        // 第一行是叶子 one → 选中即确认。
        m.dispatch(MouseEventKind::Down(MouseButton::Left), 1, 0);
        m.dispatch(MouseEventKind::Up(MouseButton::Left), 1, 0);
        assert_eq!(*log.lock().unwrap(), vec!["one".to_string()]);

        // 再点分支行(未选中)→ 只选中,不确认;再点一次(已选中)→ 展开。
        m.dispatch(MouseEventKind::Down(MouseButton::Left), 1, 1);
        m.dispatch(MouseEventKind::Up(MouseButton::Left), 1, 1);
        assert_eq!(
            *log.lock().unwrap(),
            vec!["one".to_string()],
            "分支首点不确认"
        );
    }

    // 悬停整行高亮(选中行除外),移开消失。默认 hover 样式带 BOLD。
    #[test]
    fn hovering_row_highlights_it() {
        let log: Log = Default::default();
        let mut m = Mounted::mount(crate::element!(Harness(log: log.clone())), true);

        // 第二行是分支 dir;悬停其上 → BOLD,第一行(未选中、未悬停)不受影响。
        m.dispatch(MouseEventKind::Moved, 1, 1);
        let buf = m.buffer();
        assert!(
            buf[(1, 1)].style().add_modifier.contains(Modifier::BOLD),
            "悬停行应为 BOLD"
        );
        assert!(
            !buf[(1, 0)].style().add_modifier.contains(Modifier::BOLD),
            "未悬停行不受影响"
        );

        // 移出组件 → 高亮消失。
        m.dispatch(MouseEventKind::Moved, 39, 9);
        let buf = m.buffer();
        assert!(
            !buf[(1, 1)].style().add_modifier.contains(Modifier::BOLD),
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
        m.dispatch(MouseEventKind::Moved, 1, 1);
        assert!(
            m.buffer()[(1, 1)]
                .style()
                .add_modifier
                .contains(Modifier::BOLD)
        );
        m.dispatch(MouseEventKind::Down(MouseButton::Left), 1, 1);
        m.dispatch(MouseEventKind::Up(MouseButton::Left), 1, 1);
        assert!(log.lock().unwrap().is_empty());
    }
    #[test]
    fn clicking_empty_rows_does_not_select_the_last_node() {
        let log: Log = Default::default();
        let mut m = Mounted::mount(crate::element!(Harness(log: log.clone())), true);
        m.dispatch(MouseEventKind::Down(MouseButton::Left), 1, 8);
        m.dispatch(MouseEventKind::Up(MouseButton::Left), 1, 8);
        assert!(log.lock().unwrap().is_empty());
        m.dispatch(MouseEventKind::Moved, 1, 8);
        assert!(
            !m.buffer()[(1, 1)]
                .style()
                .add_modifier
                .contains(Modifier::BOLD)
        );
    }
    #[test]
    fn movement_within_hovered_item_does_not_request_redraw() {
        let mut m = Mounted::mount(crate::element!(Harness), true);
        assert!(m.dispatch(MouseEventKind::Moved, 1, 1));
        assert!(!m.dispatch(MouseEventKind::Moved, 2, 1));
    }
}

#[cfg(test)]
mod scrollbar_tests {
    use super::*;
    use crate::{mouse_test::Mounted, prelude::*};
    use crossterm::event::{MouseButton, MouseEventKind};
    use ratatui::layout::{Constraint, Direction};
    use std::{cell::RefCell, rc::Rc};

    type TreeSlot = Rc<RefCell<Option<State<TreeState<usize>>>>>;
    type OffsetSlot = Rc<RefCell<Option<State<usize>>>>;
    #[derive(Default, Props)]
    struct PairProps {
        left: TreeSlot,
        right: OffsetSlot,
    }

    #[component]
    fn RightPane(props: &PairProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let mut offset = hooks.use_state(|| 0usize);
        *props.right.borrow_mut() = Some(offset);
        hooks.use_scrollbar(
            ScrollbarOptions {
                content_length: 100,
                viewport_length: 18,
                position: offset.get(),
                ..Default::default()
            },
            move |target| offset.set(target),
        );
        element!(View(width: Constraint::Length(40), height: Constraint::Fill(1)))
    }

    #[component]
    fn Pair(props: &PairProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let left = hooks.use_state(TreeState::default);
        *props.left.borrow_mut() = Some(left);
        element!(View(flex_direction: Direction::Horizontal) {
            TreeSelect<usize>(width: Constraint::Length(40), state: left, active: false,
                items: (0..100).map(|i| TreeItem::new_leaf(i, format!("item {i}"))).collect::<Vec<_>>(),
                block: Block::bordered(), scrollbar: ScrollbarStyle::default())
            RightPane(right: props.right.clone())
        })
    }

    #[test]
    fn adjacent_tree_and_reader_scrollbars_capture_independently_and_show_drag_feedback() {
        let left: TreeSlot = Rc::default();
        let right: OffsetSlot = Rc::default();
        let mut m = Mounted::mount_sized(
            element!(Pair(left: left.clone(), right: right.clone())),
            true,
            80,
            20,
        );
        let tree = left.borrow().unwrap();
        let reader = right.borrow().unwrap();
        assert!(!m.buffer()[(39, 2)].modifier.contains(Modifier::REVERSED));
        m.dispatch(MouseEventKind::Moved, 39, 2);
        assert!(m.buffer()[(39, 2)].modifier.contains(Modifier::BOLD));
        m.dispatch(MouseEventKind::Down(MouseButton::Left), 39, 2);
        assert!(m.buffer()[(39, 2)].modifier.contains(Modifier::REVERSED));
        // 捕获期间指针进入右面板仍只驱动左侧目录。
        m.dispatch(MouseEventKind::Drag(MouseButton::Left), 79, 18);
        assert_eq!(tree.read().get_offset(), 82);
        assert_eq!(reader.get(), 0);
        assert!(tree.read().selected().is_empty(), "拖动不确认或选择章节");
        assert!(m.buffer()[(39, 17)].modifier.contains(Modifier::REVERSED));
        m.dispatch(MouseEventKind::Up(MouseButton::Left), 79, 18);
        assert!(!m.buffer()[(39, 17)].modifier.contains(Modifier::REVERSED));
        assert!(!m.buffer()[(39, 17)].modifier.contains(Modifier::BOLD));
        m.dispatch(MouseEventKind::Down(MouseButton::Left), 79, 2);
        assert!(m.buffer()[(79, 2)].modifier.contains(Modifier::REVERSED));
        m.dispatch(MouseEventKind::Drag(MouseButton::Left), 79, 18);
        m.dispatch(MouseEventKind::Up(MouseButton::Left), 79, 18);
        assert_eq!(reader.get(), 82);
        assert_eq!(tree.read().get_offset(), 82);
    }
}
