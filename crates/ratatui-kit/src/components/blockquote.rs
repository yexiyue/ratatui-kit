use ratatui::{
    layout::Rect,
    style::{Color, Style},
};
use ratatui_kit_macros::{Props, with_layout_style};

use crate::{AnyElement, Component};

/// 引用块容器组件。
///
/// 在内容左侧渲染实心竖线（空格反色），支持嵌套深度。
/// 类似 HTML 的 `<blockquote>` 或 Markdown 的引用块。
///
/// ## 用法示例
/// ```rust,no_run
/// element!(Blockquote(depth: Some(1)) {
///     Text(text: "这是一段被引用的内容")
/// })
/// ```
#[with_layout_style]
#[derive(Props)]
pub struct BlockquoteProps<'a> {
    /// 引用深度（嵌套层级）。默认 1。
    pub depth: Option<u32>,
    /// 竖线颜色。默认 `Color::DarkGray`。
    pub prefix_color: Option<Color>,
    /// 背景颜色。默认 `Color::Rgb(25, 25, 25)` 几乎不可见。
    pub bg_color: Option<Color>,
    /// 子元素列表。
    pub children: Vec<AnyElement<'a>>,
}

impl Default for BlockquoteProps<'_> {
    fn default() -> Self {
        Self {
            depth: Some(1),
            prefix_color: Some(Color::DarkGray),
            bg_color: Some(Color::Rgb(25, 25, 25)),
            children: Vec::new(),
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

/// 引用块组件。
pub struct Blockquote {
    depth: u32,
    /// 竖线样式：空格 + 背景色 = 实心竖线
    bar_style: Style,
    /// 内容区背景样式：极淡的背景色
    bg_style: Style,
}

impl Blockquote {
    fn from_props(props: &BlockquoteProps<'_>) -> Self {
        let depth = props.depth.unwrap_or(1).max(1);
        let prefix_color = props.prefix_color.unwrap_or(Color::DarkGray);
        let bg_color = props.bg_color.unwrap_or(Color::Rgb(25, 25, 25));
        let bar_style = Style::new().bg(prefix_color);
        let bg_style = Style::new().bg(bg_color);
        Self {
            depth,
            bar_style,
            bg_style,
        }
    }

    /// 每层 1 列竖线 + 末尾 1 列间距。
    fn prefix_width(&self) -> u16 {
        self.depth as u16 + 1
    }
}

impl Component for Blockquote {
    type Props<'a> = BlockquoteProps<'a>;

    fn new(props: &Self::Props<'_>) -> Self {
        Self::from_props(props)
    }

    fn update(
        &mut self,
        props: &mut Self::Props<'_>,
        _hooks: crate::Hooks,
        updater: &mut crate::ComponentUpdater,
    ) {
        *self = Self::from_props(props);
        updater.set_layout_style(props.layout_style());
        updater.update_children(&mut props.children, None);
    }

    fn draw(&mut self, drawer: &mut crate::ComponentDrawer<'_, '_>) {
        let prefix_w = self.prefix_width();
        let area = drawer.area;

        {
            let buf = drawer.buffer_mut();
            for row in area.top()..area.bottom() {
                // 背景色 + 竖线一次遍历完成
                let mut c = area.left();
                // 先填背景色
                for col in c..area.right() {
                    buf[(col, row)].set_style(self.bg_style);
                }
                // 再画竖线
                for _d in 0..self.depth {
                    if c < area.right() {
                        buf.set_string(c, row, " ", self.bar_style);
                    }
                    c += 1;
                }
            }
        }

        drawer.area = Rect {
            x: area.x + prefix_w,
            y: area.y,
            width: area.width.saturating_sub(prefix_w),
            height: area.height,
        };
    }
}
