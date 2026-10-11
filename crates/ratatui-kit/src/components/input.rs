use crate::{
    AnyElement, ComponentTheme, EventResult, Handler, Hooks, Palette, PointerKind, PointerOptions,
    UsePointer, UsePreviousSize, UseTheme,
    components::theme::resolve_style,
    hooks::use_pointer::pointer_enabled,
    prelude::{Fragment, Positioned, Text},
};
use ratatui::{style::Style, widgets::Block};
use ratatui_kit_macros::{Props, component, element};
use unicode_width::UnicodeWidthChar;

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

#[derive(Props, Default)]
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
    /// 点击输入框定位光标的回调(参数为字符索引,与 `tui_input::InputRequest::SetCursor`
    /// 同单位)。不传则点击放行 —— Input 是受控组件,光标在调用方的 `tui_input` 状态里。
    pub on_seek: Handler<'static, usize>,
}

/// 点击显示列 → 光标字符索引:目标列落在哪个字符的显示区间内,光标就停在它**之前**
/// (点在字符上 = 在该字符处插入;与 tmux 类定位一致);超出文本末尾则停在末尾。
/// `target_cells` 是**自可视起点**(含 visual_scroll 偏移)的列数。
pub(crate) fn cursor_char_index(value: &str, target_cells: usize) -> usize {
    let mut acc = 0usize;
    let mut index = 0usize;
    for ch in value.chars() {
        let width = ch.width().unwrap_or(0);
        if target_cells < acc + width {
            return index;
        }
        acc += width;
        index += 1;
    }
    index
}

#[component]
pub fn Input(props: &mut InputProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
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

    // 点击定位光标(受控组件:算出目标字符索引交还调用方)。
    if pointer_enabled(&hooks) {
        let seek_enabled = !props.on_seek.is_default();
        let mut on_seek = props.on_seek.take();
        let input = props.input.clone();
        hooks.use_pointer(PointerOptions::default(), move |ev| {
            if seek_enabled && let PointerKind::Click { .. } = ev.kind {
                let input_width = size.width.saturating_sub(1) as usize;
                let scroll = input.visual_scroll(input_width);
                let cell = ev.position.x.saturating_sub(size.x) as usize;
                on_seek(cursor_char_index(input.value(), scroll + cell));
                EventResult::Consumed
            } else {
                EventResult::Ignored
            }
        });
    }

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
                widget(Block::new().style(cursor_style))
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // ASCII:点在字符上 → 光标停在它之前;越界停在末尾。
    #[test]
    fn cursor_index_ascii() {
        assert_eq!(cursor_char_index("abcd", 0), 0);
        assert_eq!(cursor_char_index("abcd", 2), 2);
        assert_eq!(cursor_char_index("abcd", 99), 4);
        assert_eq!(cursor_char_index("", 3), 0);
    }

    #[derive(Default, Props)]
    struct DynamicSeekProps {
        log: std::sync::Arc<std::sync::Mutex<Vec<usize>>>,
    }

    #[component]
    fn DynamicSeek(props: &DynamicSeekProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        use crate::{EventPriority, EventScope, UseEventHandler, UseState};
        use crossterm::event::{Event, KeyCode};
        let mut enabled = hooks.use_state(|| false);
        hooks.use_event_handler(EventScope::Current, EventPriority::Normal, move |event| {
            if matches!(event, Event::Key(key) if key.code == KeyCode::Char('s')) {
                enabled.set(!enabled.get());
                EventResult::Consumed
            } else {
                EventResult::Ignored
            }
        });
        let log = props.log.clone();
        let on_seek: Handler<'static, usize> = if enabled.get() {
            Handler::from(move |index| log.lock().unwrap().push(index))
        } else {
            Handler::default()
        };
        element!(Input(input: tui_input::Input::new("abc".into()), on_seek: on_seek))
    }

    #[test]
    fn click_seeking_can_be_enabled_and_disabled_after_mount() {
        use crossterm::event::{KeyCode, MouseButton, MouseEventKind};
        let log = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut mounted =
            crate::mouse_test::Mounted::mount(element!(DynamicSeek(log: log.clone())), true);
        mounted.dispatch_key(KeyCode::Char('s'));
        mounted.dispatch(MouseEventKind::Down(MouseButton::Left), 1, 0);
        mounted.dispatch(MouseEventKind::Up(MouseButton::Left), 1, 0);
        assert_eq!(log.lock().unwrap().as_slice(), &[1]);
        mounted.dispatch_key(KeyCode::Char('s'));
        mounted.dispatch(MouseEventKind::Down(MouseButton::Left), 2, 0);
        mounted.dispatch(MouseEventKind::Up(MouseButton::Left), 2, 0);
        assert_eq!(log.lock().unwrap().as_slice(), &[1]);
    }

    // 宽字符(占 2 列):点击其任一显示列都定位到该字符之前。
    #[test]
    fn cursor_index_wide_chars() {
        assert_eq!(cursor_char_index("中文", 0), 0);
        assert_eq!(cursor_char_index("中文", 1), 0);
        assert_eq!(cursor_char_index("中文", 2), 1);
        assert_eq!(cursor_char_index("a中b", 3), 2);
    }
}
