use crate::{
    AnyElement, ComponentTheme, Hooks, Palette, UsePreviousSize, UseTheme,
    components::theme::resolve_style,
    prelude::{Fragment, Positioned, Text},
};
use ratatui::{style::Style, text::Span};
use ratatui_kit_macros::{Props, component, element};

/// Input 组件的主题 slot。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputTheme {
    /// 光标块样式。
    pub cursor_style: Style,
    /// 占位符文本样式。
    pub placeholder_style: Style,
    /// 输入文本样式。
    pub style: Style,
}

impl ComponentTheme for InputTheme {
    fn from_palette(palette: &Palette) -> Self {
        Self {
            cursor_style: Style::new().bg(palette.accent).fg(palette.on_accent),
            placeholder_style: Style::new().fg(palette.placeholder),
            style: Style::new().fg(palette.fg),
        }
    }
}

impl Default for InputTheme {
    fn default() -> Self {
        Self::from_palette(&Palette::default())
    }
}

#[derive(Debug, Clone, Props, Default)]
pub struct InputProps {
    pub input: tui_input::Input,
    // 光标样式覆盖。`None` 用主题,`Some(s)` 以 `theme.patch(s)` 覆盖。
    pub cursor_style: Option<Style>,
    pub placeholder: String,
    // 占位符样式覆盖。`None` 用主题,`Some(s)` 以 `theme.patch(s)` 覆盖。
    pub placeholder_style: Option<Style>,
    // 输入文本样式覆盖。`None` 用主题,`Some(s)` 以 `theme.patch(s)` 覆盖。
    pub style: Option<Style>,
    pub hide_cursor: bool,
}

#[component]
pub fn Input(props: &InputProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    // 主题解析:theme slot 铺底,props 的 Option<Style> 在上 patch(None → 用主题)。
    let theme = hooks.use_component_theme::<InputTheme>();
    let cursor_style = resolve_style(theme.cursor_style, props.cursor_style);
    let placeholder_style = resolve_style(theme.placeholder_style, props.placeholder_style);
    let style = resolve_style(theme.style, props.style);

    let size = hooks.use_previous_size();
    let input = &props.input;
    let input_width = size.width.saturating_sub(1) as usize;
    let scroll = if props.hide_cursor || input_width == 0 {
        0
    } else {
        input.visual_scroll(input_width)
    };
    let text = if input.value().is_empty() {
        props.placeholder.clone()
    } else {
        input.value().to_string()
    };

    let x = input.visual_cursor().max(scroll) - scroll;

    let position = (size.x + x as u16, size.y);

    element!(Fragment {
        Text(
            text:text,
            style: if input.value().is_empty() {
                placeholder_style
            }else{
                style
            },
            scroll:(0, scroll as u16),
        )
        if !props.hide_cursor {
            Positioned(
                x: position.0.min(size.x + size.width.saturating_sub(1)),
                y: position.1.min(size.y + size.height),
                width: 1u16,
                height: 1u16,
            ){
                widget(Span::from(" ").style(cursor_style))
            }
        }
    })
}
