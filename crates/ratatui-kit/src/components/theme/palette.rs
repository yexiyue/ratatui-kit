// Palette:主题系统的唯一颜色真源。所有内置组件的默认样式都从这里派生,
// 因此改这一处即可统一全部组件观感。

use ratatui::style::Color;

/// 语义色板——主题系统的唯一颜色真源。
///
/// 每个内置组件的 `FooTheme::from_palette` 只从这里取色;改这一处即可统一全部组件观感。
/// 标注 `#[non_exhaustive]`:后续新增语义色不构成对下游的破坏性变更。下游经
/// [`Palette::default`] + 字段修改构造,而非结构体字面量。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Palette {
    /// 主前景色(正文文字)。
    pub fg: Color,
    /// 弱化前景(次要文字、禁用态)。
    pub fg_dim: Color,
    /// 主背景色。
    pub bg: Color,
    /// 面板/容器表面色(比 `bg` 略微抬升的层)。
    pub surface: Color,
    /// 遮罩层底色(模态背景)。
    pub overlay: Color,
    /// 强调色(高亮、激活边框、选中背景)。
    pub accent: Color,
    /// 强调底色其上的前景(如高亮行的文字色,与 `accent`/`selection` 配对)。
    pub on_accent: Color,
    /// 选中项背景。
    pub selection: Color,
    /// 常态边框色。
    pub border: Color,
    /// 激活/聚焦边框色。
    pub border_active: Color,
    /// 成功语义色。
    pub success: Color,
    /// 警告语义色。
    pub warning: Color,
    /// 错误语义色。
    pub error: Color,
    /// 信息语义色。
    pub info: Color,
    /// 占位符/提示文字色。
    pub placeholder: Color,
}

impl Default for Palette {
    /// 一套自洽的默认配色:交互/高亮走 accent(cyan)族,边框常态 dim、激活 accent,
    /// 语义色 success/warning/error/info = green/yellow/red/blue,前景/背景默认跟随终端(`Reset`)。
    fn default() -> Self {
        Self {
            fg: Color::Reset,
            fg_dim: Color::DarkGray,
            bg: Color::Reset,
            surface: Color::Reset,
            overlay: Color::Reset,
            accent: Color::Cyan,
            on_accent: Color::Black,
            selection: Color::Cyan,
            border: Color::DarkGray,
            border_active: Color::Cyan,
            success: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,
            info: Color::Blue,
            placeholder: Color::DarkGray,
        }
    }
}
