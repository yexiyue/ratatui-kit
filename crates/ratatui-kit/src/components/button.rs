// Button 组件:左键按下(Down)即触发的可点击按钮。
//
// - `on_press` 在左键 `Down` 命中时触发并消费事件(终端惯例,不等释放)。
// - 内置 hover 高亮(需应用启用鼠标捕获);`disabled` 时灰显、不触发也不拦截下层交互。
// - `variant` 决定基础样式(经 [`ButtonTheme`] 从 [`Palette`] 派生,可整体 override)。

use crossterm::event::MouseButton;
use ratatui::{
    layout::{Alignment, Constraint},
    style::{Modifier, Style},
    text::Line,
};

use crate::{
    AnyElement, ComponentTheme, Handler, Hooks, MouseButtons, Palette, PointerKind, PointerOptions,
    UseHover, UsePointer, UseTheme,
    components::{Border, Text, theme::resolve_style},
    input::EventResult,
};
use ratatui_kit_macros::{Props, component};

/// Button 组件的主题 slot。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ButtonTheme {
    /// 主按钮样式(默认反白于 accent)。
    pub primary_style: Style,
    /// 次按钮样式。
    pub secondary_style: Style,
    /// 危险操作样式(默认反白于 error)。
    pub danger_style: Style,
    /// 禁用样式。
    pub disabled_style: Style,
    /// hover 时附加的修饰(默认 `BOLD`)。
    pub hover_modifier: Modifier,
}

impl ComponentTheme for ButtonTheme {
    fn from_palette(palette: &Palette) -> Self {
        Self {
            primary_style: Style::new().bg(palette.accent).fg(palette.on_accent),
            secondary_style: Style::new().bg(palette.surface).fg(palette.fg),
            danger_style: Style::new().bg(palette.error).fg(palette.on_accent),
            disabled_style: Style::new().bg(palette.bg).fg(palette.placeholder),
            hover_modifier: Modifier::BOLD,
        }
    }
}

impl Default for ButtonTheme {
    fn default() -> Self {
        Self::from_palette(&Palette::default())
    }
}

/// 按钮变体。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonVariant {
    /// 主操作(默认)。
    #[default]
    Primary,
    /// 次要操作。
    Secondary,
    /// 危险操作。
    Danger,
}

#[derive(Default, Props)]
pub struct ButtonProps {
    /// 按钮文字。
    pub label: String,
    /// 左键按下时的回调。
    pub on_press: Handler<'static, ()>,
    /// 变体。
    pub variant: ButtonVariant,
    /// 禁用:不触发回调、不参与 hover 高亮,事件放行(不拦截下层交互)。
    pub disabled: bool,
    /// 总宽(含边框)。`None` = 自适应(标签显示宽 + 4)。
    pub width: Option<u16>,
    /// 样式覆盖(在变体基础上 patch;`disabled` 优先于覆盖)。
    pub style: Option<Style>,
}

// 合成按钮样式:变体 → 覆盖 patch → 禁用优先 → hover 附加修饰。
fn button_style(
    theme: &ButtonTheme,
    variant: ButtonVariant,
    disabled: bool,
    hovered: bool,
    override_style: Option<Style>,
) -> Style {
    if disabled {
        return theme.disabled_style;
    }
    let base = match variant {
        ButtonVariant::Primary => theme.primary_style,
        ButtonVariant::Secondary => theme.secondary_style,
        ButtonVariant::Danger => theme.danger_style,
    };
    let style = resolve_style(base, override_style);
    if hovered {
        style.add_modifier(theme.hover_modifier)
    } else {
        style
    }
}

// 边框与标签文字同取主题背景色,标签去掉填充(与 ConfirmModal 按钮同款)。
fn split_border_label(style: Style) -> (Style, Style) {
    let mut label = style;
    let mut border = style;
    if let Some(bg) = style.bg {
        label.fg = Some(bg);
        label.bg = None;
        border.fg = Some(bg);
    }
    (border, label)
}

#[component]
pub fn Button(props: &mut ButtonProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    // hover 始终注册(hook 顺序稳定);disabled 时仅不参与高亮。
    let hovered = hooks.use_hover();

    // 左键 Down 触发并消费;disabled 放行事件。
    let mut on_press = props.on_press.take();
    let disabled = props.disabled;
    hooks.use_pointer(
        PointerOptions {
            buttons: MouseButtons::LEFT,
            track_move: false,
            ..Default::default()
        },
        move |ev| {
            if !disabled && matches!(ev.kind, PointerKind::Down(MouseButton::Left)) {
                on_press(());
                EventResult::Consumed
            } else {
                EventResult::Ignored
            }
        },
    );

    let theme = hooks.use_component_theme::<ButtonTheme>();
    let hovered = hovered && !props.disabled;
    let style = button_style(&theme, props.variant, props.disabled, hovered, props.style);
    // 禁用态无填充背景,不做 fg/bg 交换(避免把 placeholder 色改写成背景色)。
    let (border_style, label_style) = if props.disabled {
        (style, style)
    } else {
        split_border_label(style)
    };

    let width = props
        .width
        .unwrap_or_else(|| (Line::from(props.label.as_str()).width() as u16).saturating_add(4));
    let label = format!(" {} ", props.label);

    crate::element!(
        Border(
            width: Constraint::Length(width),
            height: Constraint::Length(3),
            border_style: border_style,
        ) {
            Text(
                text: label,
                style: label_style,
                alignment: Alignment::Center,
            )
        }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::{Color, Modifier};

    // ---- 样式合成单测 ----

    fn theme() -> ButtonTheme {
        ButtonTheme::default()
    }

    #[test]
    fn style_by_variant() {
        let t = theme();
        assert_eq!(
            button_style(&t, ButtonVariant::Primary, false, false, None),
            t.primary_style
        );
        assert_eq!(
            button_style(&t, ButtonVariant::Secondary, false, false, None),
            t.secondary_style
        );
        assert_eq!(
            button_style(&t, ButtonVariant::Danger, false, false, None),
            t.danger_style
        );
    }

    #[test]
    fn style_disabled_wins_over_everything() {
        let t = theme();
        let override_style = Style::new().fg(Color::Red);
        assert_eq!(
            button_style(&t, ButtonVariant::Primary, true, true, Some(override_style)),
            t.disabled_style
        );
    }

    #[test]
    fn style_hover_adds_modifier() {
        let t = theme();
        let hovered = button_style(&t, ButtonVariant::Primary, false, true, None);
        assert_eq!(
            hovered.add_modifier,
            t.primary_style.add_modifier | Modifier::BOLD
        );
    }

    #[test]
    fn style_override_patches() {
        let t = theme();
        let style = button_style(
            &t,
            ButtonVariant::Secondary,
            false,
            false,
            Some(Style::new().fg(Color::Green)),
        );
        assert_eq!(style.fg, Some(Color::Green));
        assert_eq!(style.bg, t.secondary_style.bg);
    }

    // 自适应宽度
    #[test]
    fn auto_width_is_label_plus_padding() {
        assert_eq!(Line::from("删除").width() as u16 + 4, 8); // CJK 按 2 格计
    }

    // ---- 交互与渲染(共享鼠标 harness) ----

    mod interaction {
        use super::*;
        use crate::{Palette, components::View, mouse_test::Mounted};
        use crossterm::event::{MouseButton, MouseEventKind};
        use ratatui::layout::Direction;
        use ratatui_kit_macros::{Props, component};
        use std::sync::{Arc, Mutex};

        // Handler<'static, ()> 要求闭包 Send + Sync(与既有回调 prop 一致),用 Arc<Mutex> 记录。
        #[derive(Clone, Default)]
        struct PressLog(Arc<Mutex<Vec<String>>>);

        #[derive(Default, Props)]
        struct ButtonAppProps {
            log: PressLog,
            disabled: bool,
        }

        #[component]
        fn ButtonApp(props: &ButtonAppProps, _hooks: Hooks) -> impl Into<AnyElement<'static>> {
            let log = props.log.clone();
            crate::element!(View(flex_direction: Direction::Horizontal) {
                Button(
                    label: "OK",
                    on_press: move |()| log.0.lock().unwrap().push("pressed".to_string()),
                    disabled: props.disabled,
                )
                Filler
            })
        }

        #[component]
        fn Filler() -> impl Into<AnyElement<'static>> {
            crate::element!(Text(text: "............"))
        }

        // 左键 Down 命中即触发一次;后续 Up 不重复触发
        #[test]
        fn press_fires_once_on_down() {
            let log = PressLog::default();
            let mut m = Mounted::mount(
                crate::element!(ButtonApp(log: log.clone(), disabled: false)),
                true,
            );
            m.dispatch(MouseEventKind::Down(MouseButton::Left), 2, 1);
            m.dispatch(MouseEventKind::Up(MouseButton::Left), 2, 1);
            assert_eq!(*log.0.lock().unwrap(), vec!["pressed".to_string()]);
        }

        // 区域外按下不触发
        #[test]
        fn press_outside_does_not_fire() {
            let log = PressLog::default();
            let mut m = Mounted::mount(
                crate::element!(ButtonApp(log: log.clone(), disabled: false)),
                true,
            );
            m.dispatch(MouseEventKind::Down(MouseButton::Left), 30, 5);
            assert!(log.0.lock().unwrap().is_empty());
        }

        // disabled 不触发
        #[test]
        fn disabled_press_does_not_fire() {
            let log = PressLog::default();
            let mut m = Mounted::mount(
                crate::element!(ButtonApp(log: log.clone(), disabled: true)),
                true,
            );
            m.dispatch(MouseEventKind::Down(MouseButton::Left), 2, 1);
            assert!(log.0.lock().unwrap().is_empty());
        }

        // 渲染:标签与边框取主题色;disabled 用 placeholder 色
        #[test]
        fn render_variants_take_theme_colors() {
            let palette = Palette::default();
            let mut m = Mounted::mount(
                crate::element!(ButtonApp(log: PressLog::default(), disabled: false)),
                true,
            );
            let buf = m.buffer();
            // Button 自适应宽 6(label "OK" 2 + 4),高 3,位于 (0,0)。
            assert_eq!(buf[(2, 1)].symbol(), "O");
            assert_eq!(buf[(3, 1)].symbol(), "K");
            assert_eq!(buf[(2, 1)].style().fg, Some(palette.accent));

            let mut m = Mounted::mount(
                crate::element!(ButtonApp(log: PressLog::default(), disabled: true)),
                true,
            );
            let buf = m.buffer();
            assert_eq!(buf[(2, 1)].style().fg, Some(palette.placeholder));
        }
    }
}
