use ratatui::{
    style::Style,
    widgets::{Block, Scrollbar},
};
use ratatui_kit::{Component, Props, State};
use std::hash::Hash;
use tui_tree_widget::{TreeItem, TreeState};

/// 树形组件的属性定义
#[derive(Debug, Props, Clone)]
pub struct TreeSelect<T>
where
    T: Sync + Send + Clone + Eq + Hash + 'static,
{
    /// 树形组件的状态，可选
    pub state: Option<State<TreeState<T>>>,

    /// 树形节点项列表
    pub items: Vec<TreeItem<'static, T>>,

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
    /// 可选的边框块
    pub block: Option<Block<'static>>,
}

impl<T> Default for TreeSelect<T>
where
    T: Sync + Send + Clone + Eq + Hash,
{
    fn default() -> Self {
        Self {
            state: None,
            items: vec![],
            scrollbar: None,
            style: Style::new(),
            highlight_style: Style::new(),
            highlight_symbol: "",
            node_closed_symbol: "\u{25b6} ", // 向右箭头
            node_open_symbol: "\u{25bc} ",   // 向下箭头
            node_no_children_symbol: "  ",
            block: None,
        }
    }
}

impl<T> Component for TreeSelect<T>
where
    T: Sync + Send + Clone + Eq + Hash + Unpin + 'static,
{
    type Props<'a>
        = TreeSelect<T>
    where
        Self: 'a;

    /// 根据属性创建新的树形组件实例
    fn new(props: &Self::Props<'_>) -> Self {
        props.clone()
    }

    /// 绘制树形组件
    fn draw(&mut self, drawer: &mut ratatui_kit::ComponentDrawer<'_, '_>) {
        let mut tree = tui_tree_widget::Tree::new(&self.items)
            .unwrap()
            .style(self.style)
            .highlight_style(self.highlight_style)
            .highlight_symbol(self.highlight_symbol)
            .node_closed_symbol(self.node_closed_symbol)
            .node_open_symbol(self.node_open_symbol)
            .node_no_children_symbol(self.node_no_children_symbol)
            .experimental_scrollbar(self.scrollbar.clone());

        if let Some(block) = &self.block {
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
