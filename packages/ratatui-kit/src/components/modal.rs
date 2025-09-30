//! Modal 组件：模态弹窗，支持遮罩、居中/自定义位置、尺寸、样式等。
//!
//! ## 用法示例
//! ```rust
//! element!(Modal(
//!     open: open.get(),
//!     width: Constraint::Percentage(60),
//!     height: Constraint::Percentage(60),
//!     style: Style::default().dim(),
//! ){
//!     Border(top_title: Some(Line::from("弹窗内容"))) {
//!         // ...子内容
//!     }
//! })
//! ```
//! 通过 `open` 控制显示，`placement` 控制弹窗位置，`width/height` 控制尺寸。

use ratatui::{
    layout::{Constraint, Flex, Layout, Margin, Offset},
    style::Style,
    widgets::{Block, Clear, Widget},
};
use ratatui_kit_macros::{Props, with_layout_style};

use crate::{AnyElement, Component, layout_style::LayoutStyle};

#[derive(Default, Clone, Copy)]
/// 弹窗位置枚举。
pub enum Placement {
    Top,
    TopLeft,
    TopRight,
    Bottom,
    BottomLeft,
    BottomRight,
    #[default]
    Center,
    Left,
    Right,
}

impl Placement {
    pub fn to_flex(&self) -> [Flex; 2] {
        match self {
            Placement::Top => [Flex::Start, Flex::Center],
            Placement::TopLeft => [Flex::Start, Flex::Start],
            Placement::TopRight => [Flex::Start, Flex::End],
            Placement::Bottom => [Flex::End, Flex::Center],
            Placement::BottomLeft => [Flex::End, Flex::Start],
            Placement::BottomRight => [Flex::End, Flex::End],
            Placement::Center => [Flex::Center, Flex::Center],
            Placement::Left => [Flex::Center, Flex::Start],
            Placement::Right => [Flex::Center, Flex::End],
        }
    }
}

#[with_layout_style(margin, offset, width, height)]
#[derive(Default, Props)]
/// Modal 组件属性。
pub struct ModalProps<'a> {
    /// 弹窗内容。
    pub children: Vec<AnyElement<'a>>,
    /// 弹窗样式。
    pub style: Style,
    /// 弹窗位置。
    pub placement: Placement,
    /// 是否显示弹窗。
    pub open: bool,
}

/// Modal 组件实现。
pub struct Modal {
    pub open: bool,
    pub margin: Margin,
    pub offset: Offset,
    pub width: Constraint,
    pub height: Constraint,
    pub placement: Placement,
    pub style: Style,
}

impl Component for Modal {
    type Props<'a> = ModalProps<'a>;
    fn new(props: &Self::Props<'_>) -> Self {
        Modal {
            open: props.open,
            margin: props.margin,
            offset: props.offset,
            width: props.width,
            height: props.height,
            style: props.style,
            placement: props.placement,
        }
    }

    fn update(
        &mut self,
        props: &mut Self::Props<'_>,
        _hooks: crate::Hooks,
        updater: &mut crate::ComponentUpdater,
    ) {
        self.open = props.open;
        self.margin = props.margin;
        self.offset = props.offset;
        self.width = props.width;
        self.height = props.height;
        self.style = props.style;
        self.placement = props.placement;

        if self.open {
            updater.update_children(props.children.iter_mut(), None);
        }

        updater.set_layout_style(LayoutStyle {
            width: Constraint::Length(0),
            height: Constraint::Length(0),
            ..Default::default()
        });
    }

    fn draw(&mut self, drawer: &mut crate::ComponentDrawer<'_, '_>) {
        if self.open {
            // 根据终端尺寸计算弹窗尺寸和位置
            let area = drawer.buffer_mut().area();
            let area = area.inner(self.margin).offset(self.offset);

            let block = Block::default().style(self.style);
            block.render(area, drawer.buffer_mut());

            let [v, h] = self.placement.to_flex();

            let vertical = Layout::vertical([self.height]).flex(v).split(area)[0];
            let horizontal = Layout::horizontal([self.width]).flex(h).split(vertical)[0];

            // 清空弹窗区域
            Clear.render(horizontal, drawer.buffer_mut());
            drawer.area = horizontal;
        }
    }
}
