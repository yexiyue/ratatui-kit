//! Border 组件：为内容添加可定制的边框、标题、内边距等。
//!
//! 常用于包裹内容、分组、突出显示等场景。
//!
//! ## 用法示例
//! ```rust
//! element!(Border(
//!     border_style: Style::default().blue(),
//!     top_title: Some(Line::from("标题")),
//!     padding: Padding::new(1, 1, 0, 0),
//! ){
//!     ChildComponent()
//! })
//! ```
//! 支持自定义边框样式、边框字符集、上下标题、内边距等属性。

use ratatui::{
    symbols::border,
    text::Line,
    widgets::{Block, Padding, Widget},
};
use ratatui_kit_macros::{Props, with_layout_style};

use crate::{AnyElement, Component};

#[with_layout_style]
#[derive(Props)]
/// Border 组件属性。
pub struct BorderProps<'a> {
    /// 内边距。
    pub padding: Padding,
    /// 边框样式。
    pub border_style: ratatui::style::Style,
    /// 显示哪些边。
    pub borders: ratatui::widgets::Borders,
    /// 边框字符集。
    pub border_set: border::Set,
    /// 整体样式。
    pub style: ratatui::style::Style,
    /// 子元素列表。
    pub children: Vec<AnyElement<'a>>,
    /// 顶部标题。
    pub top_title: Option<Line<'static>>,
    /// 底部标题。
    pub bottom_title: Option<Line<'static>>,
}

impl Default for BorderProps<'_> {
    fn default() -> Self {
        Self {
            padding: Padding::default(),
            border_style: ratatui::style::Style::default(),
            borders: ratatui::widgets::Borders::ALL,
            children: Vec::new(),
            border_set: border::Set::default(),
            style: ratatui::style::Style::default(),
            top_title: None,
            bottom_title: None,
            margin: Default::default(),
            offset: Default::default(),
            width: Default::default(),
            height: Default::default(),
            gap: Default::default(),
            flex_direction: Default::default(),
            justify_content: Default::default(),
        }
    }
}

/// Border 组件实现。
pub struct Border {
    pub padding: Padding,
    pub border_style: ratatui::style::Style,
    pub borders: ratatui::widgets::Borders,
    pub border_set: border::Set,
    pub style: ratatui::style::Style,
    pub top_title: Option<Line<'static>>,
    pub bottom_title: Option<Line<'static>>,
}

impl Component for Border {
    type Props<'a> = BorderProps<'a>;

    /// 根据属性创建 Border 组件实例
    fn new(props: &Self::Props<'_>) -> Self {
        Self {
            padding: props.padding,
            border_style: props.border_style,
            borders: props.borders,
            border_set: props.border_set,
            style: props.style,
            top_title: props.top_title.clone(),
            bottom_title: props.bottom_title.clone(),
        }
    }

    /// 根据最新属性和子组件更新自身状态
    fn update(
        &mut self,
        props: &mut Self::Props<'_>,
        _hooks: crate::Hooks,
        updater: &mut crate::ComponentUpdater,
    ) {
        // 获取布局属性
        let layout_style = props.layout_style();
        // 用新属性重建自身
        *self = Self {
            padding: props.padding,
            border_style: props.border_style,
            borders: props.borders,
            border_set: props.border_set,
            style: props.style,
            top_title: props.top_title.clone(),
            bottom_title: props.bottom_title.clone(),
        };
        // 设置布局样式
        updater.set_layout_style(layout_style);
        // 更新子组件
        updater.update_children(&mut props.children, None);
    }

    /// 渲染 Border 组件
    fn draw(&mut self, drawer: &mut crate::ComponentDrawer<'_, '_>) {
        // 构建 Block，设置样式、边框、内边距等
        let mut block = Block::new()
            .style(self.style)
            .borders(self.borders)
            .border_set(self.border_set)
            .border_style(self.border_style)
            .padding(self.padding);

        // 设置顶部标题（如有）
        if let Some(top_title) = &self.top_title {
            block = block.title_top(top_title.clone());
        }

        // 设置底部标题（如有）
        if let Some(bottom_title) = &self.bottom_title {
            block = block.title_bottom(bottom_title.clone());
        }

        // 计算内容区域
        let inner_area = block.inner(drawer.area);
        // 渲染边框
        block.render(drawer.area, drawer.buffer_mut());
        // 更新绘制区域为内容区，供子组件使用
        drawer.area = inner_area;
    }
}
