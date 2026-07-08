// Border 组件：为内容添加可定制的边框、标题、内边距等。
//
// 常用于包裹内容、分组、突出显示等场景。
//
// ## 用法示例
// ```rust
// element!(Border(
//     border_style: Style::default().blue(),   // Some(_) 覆盖主题;省略则用主题
//     top_title: Some(Line::from("标题")),
//     padding: Padding::new(1, 1, 0, 0),
// ){
//     ChildComponent()
// })
// ```
// 样式默认来自主题(`BorderTheme`,从 `Palette` 派生);`border_style` / `style` 为 `Option<Style>`,
// `None` 用主题、`Some(s)` 以 `theme.patch(s)` 覆盖。

use ratatui::{
    style::Style,
    symbols::border,
    text::Line,
    widgets::{Block, Padding, Widget},
};
use ratatui_kit_macros::{Props, with_layout_style};

use crate::{AnyElement, Component, ComponentTheme, Palette, components::theme::resolve_style};

/// Border 组件的主题 slot。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BorderTheme {
    /// 边框颜色样式。
    pub border_style: Style,
    /// 整体区域样式。
    pub style: Style,
}

impl ComponentTheme for BorderTheme {
    fn from_palette(palette: &Palette) -> Self {
        Self {
            border_style: Style::new().fg(palette.border),
            style: Style::new(),
        }
    }
}

impl Default for BorderTheme {
    fn default() -> Self {
        Self::from_palette(&Palette::default())
    }
}

#[with_layout_style]
#[derive(Props)]
// Border 组件属性。
pub struct BorderProps<'a> {
    // 内边距。
    pub padding: Padding,
    // 边框样式覆盖。`None` 用主题,`Some(s)` 以 `theme.patch(s)` 覆盖。
    pub border_style: Option<Style>,
    // 显示哪些边。
    pub borders: ratatui::widgets::Borders,
    // 边框字符集。
    pub border_set: border::Set<'static>,
    // 整体样式覆盖。`None` 用主题,`Some(s)` 以 `theme.patch(s)` 覆盖。
    pub style: Option<Style>,
    // 子元素列表。
    pub children: Vec<AnyElement<'a>>,
    // 顶部标题。可直接传 `Line`(经宏 `.into()` + std `From<T> for Option<T>` 自动 `Some`)或 `Option<Line>`。
    pub top_title: Option<Line<'static>>,
    // 底部标题。可直接传 `Line`(自动 `Some`)或 `Option<Line>`。
    pub bottom_title: Option<Line<'static>>,
}

impl Default for BorderProps<'_> {
    fn default() -> Self {
        Self {
            padding: Padding::default(),
            border_style: None,
            borders: ratatui::widgets::Borders::ALL,
            children: Vec::new(),
            border_set: border::Set::default(),
            style: None,
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

// Border 组件实现。持有**已解析**的样式(主题 patch 过 props),draw 直接用。
pub struct Border {
    pub padding: Padding,
    pub border_style: Style,
    pub borders: ratatui::widgets::Borders,
    pub border_set: border::Set<'static>,
    pub style: Style,
    pub top_title: Option<Line<'static>>,
    pub bottom_title: Option<Line<'static>>,
}

impl Border {
    // 从 props 派生非样式字段(样式在 update 中经主题解析后写入)。
    fn from_props(props: &BorderProps<'_>) -> Self {
        Self {
            padding: props.padding,
            border_style: Style::default(),
            borders: props.borders,
            border_set: props.border_set,
            style: Style::default(),
            top_title: props.top_title.clone(),
            bottom_title: props.bottom_title.clone(),
        }
    }
}

impl Component for Border {
    type Props<'a> = BorderProps<'a>;

    // 根据属性创建 Border 组件实例(样式待 update 解析)
    fn new(props: &Self::Props<'_>) -> Self {
        Self::from_props(props)
    }

    // 根据最新属性和子组件更新自身状态
    fn update(
        &mut self,
        props: &mut Self::Props<'_>,
        _hooks: crate::Hooks,
        updater: &mut crate::ComponentUpdater,
    ) {
        *self = Self::from_props(props);
        // 主题解析:theme slot 铺底,props 的 Option<Style> 在上 patch(None → 用主题)。
        // use_component_theme 返回 owned 值、读后即弃守卫,不与后续 &mut updater 冲突。
        let theme = updater.use_component_theme::<BorderTheme>();
        self.border_style = resolve_style(theme.border_style, props.border_style);
        self.style = resolve_style(theme.style, props.style);
        // 布局与子节点收尾保持显式（不并入 from_props，后者只构造自身状态）。
        updater.set_layout_style(props.layout_style());
        updater.update_children(&mut props.children, None);
    }

    // 渲染 Border 组件
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
