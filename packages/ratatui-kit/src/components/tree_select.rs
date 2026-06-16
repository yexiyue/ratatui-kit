use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    style::Style,
    widgets::{Block, Scrollbar},
};
use ratatui_kit::{
    Component, Handler, Props, State, UseEffect, UseEventHandler, UseState,
    input::{EventPriority, EventResult, EventScope},
    with_layout_style,
};
use std::hash::Hash;
use tui_tree_widget::{TreeItem, TreeState};

/// 树形组件的属性定义
#[with_layout_style(margin, offset, width, height)]
#[derive(Props)]
pub struct TreeSelectProps<T>
where
    T: Sync + Send + Clone + Eq + Hash + 'static,
{
    /// 树形组件的状态，可选
    pub state: Option<State<TreeState<T>>>,

    /// 树形节点项列表
    pub items: Vec<TreeItem<'static, T>>,
    /// 是否启用内置键盘交互。默认关闭，以保持原渲染型组件语义。
    pub active: bool,
    /// 默认选中的节点路径；例如 `["components", "input"]`。
    pub default_selection: Vec<T>,
    /// 当前项确认选择时触发。
    pub on_select: Handler<'static, T>,

    /// 滚动条组件，可选
    pub scrollbar: Option<Scrollbar<'static>>,
    /// 用于组件的基础样式
    pub style: Style,

    /// 用于渲染选中项的样式
    pub highlight_style: Style,
    /// 显示在选中项前面的符号（会将所有项右移）
    pub highlight_symbol: &'static str,

    /// 显示在已关闭节点前面的符号（子节点当前不可见）
    pub node_closed_symbol: &'static str,
    /// 显示在已打开节点前面的符号（子节点当前可见）
    pub node_open_symbol: &'static str,
    /// 显示在没有子节点的节点前面的符号
    pub node_no_children_symbol: &'static str,
    /// 可选的边框块。
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
            style: Style::new(),
            highlight_style: Style::new(),
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

fn sync_default_tree_selection<T>(
    state: &mut TreeState<T>,
    last_default_selection: &mut Option<Vec<T>>,
    default_selection: &[T],
) where
    T: Clone + Eq + Hash,
{
    let default_changed = last_default_selection.as_deref() != Some(default_selection);

    if default_changed {
        *last_default_selection = Some(default_selection.to_vec());
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

/// 树形组件实现。
pub struct TreeSelect<T>
where
    T: Sync + Send + Clone + Eq + Hash + 'static,
{
    state: Option<State<TreeState<T>>>,
    items: Vec<TreeItem<'static, T>>,
    scrollbar: Option<Scrollbar<'static>>,
    style: Style,
    highlight_style: Style,
    highlight_symbol: &'static str,
    node_closed_symbol: &'static str,
    node_open_symbol: &'static str,
    node_no_children_symbol: &'static str,
    block: Option<Block<'static>>,
    tree_is_valid: bool,
}

impl<T> TreeSelect<T>
where
    T: Sync + Send + Clone + Eq + Hash + 'static,
{
    fn from_props(props: &TreeSelectProps<T>) -> Self {
        Self {
            state: props.state,
            items: props.items.clone(),
            scrollbar: props.scrollbar.clone(),
            style: props.style,
            highlight_style: props.highlight_style,
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

    /// 根据属性创建新的树形组件实例
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
        let mut on_select = props.on_select.take();
        hooks.use_event_handler(EventScope::Current, EventPriority::Normal, move |event| {
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
                        on_select(selected);
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

    /// 绘制树形组件
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
            .node_no_children_symbol(self.node_no_children_symbol)
            .experimental_scrollbar(self.scrollbar.clone());

        if let Some(block) = self.block.as_ref() {
            tree = tree.block(block.clone());
        }

        if let Some(state) = &mut self.state {
            // 渲染有状态的树形组件
            drawer.render_stateful_widget(tree, drawer.area, &mut state.write_no_update());
        } else {
            // 渲染无状态的树形组件
            drawer.render_widget(tree, drawer.area);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn empty_default_selection_clears_selected_path() {
        let mut state = TreeState::<&'static str>::default();
        state.select(vec!["components", "select"]);
        let mut last_default = Some(vec!["components", "select"]);

        sync_default_tree_selection(&mut state, &mut last_default, &[]);

        assert!(state.selected().is_empty());
    }
}
