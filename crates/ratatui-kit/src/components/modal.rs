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

use crate::{
    AnyElement, Component, Context, SystemContext,
    input::{CurrentLayer, InputLayer},
    layout_style::LayoutStyle,
};

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
    /// 外部注入的输入层句柄（父组件已 `use_input_layer` 时)。
    ///
    /// `None` → Modal 内部自开层（handler 全在 Modal 子树内的常见场景)；
    /// `Some(h)` → 复用父级已登记的层（不重复登记)，仅向子树注入 `CurrentLayer`——
    /// 用于「handler 注册在 Modal 父组件」的场景（父 `use_input_layer` + `use_event_handler(Layer(h))`)。
    ///
    /// **Footgun**：走 `Layer(h)` 路径时必须把 `h` 传进来，否则 Modal 自开新层会截断 `h` → 父级 handler 失聪。
    pub layer: Option<InputLayer>,
    /// 是否截断更低层。`None` 视作 `true`（模态独占输入)；非阻塞浮层可设 `Some(false)`。
    pub blocks_lower: Option<bool>,
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
            let blocks = props.blocks_lower.unwrap_or(true);
            // 借用纪律：取 SystemContext 守卫拿 layer id 后【立即 drop】,再 update_children,
            // 否则子树组件访问 SystemContext(use_input_layer / use_exit)会撞 AlreadyBorrowed。
            let layer_id = match props.layer {
                // 外部已登记该层（父级 use_input_layer)：Modal 不重复 push,仅注入给子树。
                Some(h) => h.id,
                // 内部自开层（handler 全在 Modal 子树内)：push 一个独占层。
                None => {
                    let mut sys = updater
                        .get_context_mut::<SystemContext>()
                        .expect("SystemContext 缺失(根 context 必有)");
                    sys.input.push_layer(true, blocks).id
                }
            };

            // 给子树注入 CurrentLayer:子树内 use_event_handler(Current) 自动归属本层。
            updater.update_children(
                props.children.iter_mut(),
                Some(Context::owned(CurrentLayer(layer_id))),
            );
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
