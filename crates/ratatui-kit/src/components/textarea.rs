// TextArea 组件:软换行的多行文本输入。
//
// 在 `tui-input` 之上自行实现显示层换行(不依赖已停更的 tui-textarea):
// 逻辑值是普通字符串(可含 `\n`),按组件宽度做贪心软换行 —— 空白后优先断行、
// 超宽单词硬拆;Up/Down 在**显示行**间移动(列宽保持),Home/End 落在显示行首/尾,
// 内容超出高度时竖向滚动跟随光标。受控组件:`value` + `on_change`;
// `multiline` 决定 Enter 是插入换行还是提交(`on_submit`)。
//
// ## 用法示例
// ```rust
// let value = hooks.use_state(String::new);
// element!(TextArea(
//     value: value.read().to_string(),
//     is_focus: true,
//     on_change: move |next: String| value.set(next),
//     multiline: true,
//     placeholder: "请输入…".to_string(),
// ))
// ```

use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{style::Style, text::Line, widgets::Block};
use ratatui_kit_macros::{Props, component, element};
use std::{cell::RefCell, rc::Rc};
use tui_input::{Input, InputRequest, StateChanged};
use unicode_width::UnicodeWidthChar;

use crate::{
    AnyElement, ComponentTheme, EventResult, Handler, Hook, Hooks, Palette, PointerKind,
    PointerOptions, UseEffect, UseEventHandler, UsePointer, UsePreviousSize, UseState, UseTheme,
    components::theme::resolve_style,
    components::{Fragment, Positioned, Text},
    hooks::use_pointer::pointer_enabled,
    input::{EventPriority, EventScope},
};

/// TextArea 组件的主题 slot(与 `InputTheme` 同构)。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextAreaTheme {
    /// 光标块样式。
    pub cursor_style: Style,
    /// 占位符文本样式。
    pub placeholder_style: Style,
    /// 正文样式。
    pub style: Style,
}

impl ComponentTheme for TextAreaTheme {
    fn from_palette(palette: &Palette) -> Self {
        Self {
            cursor_style: Style::new().bg(palette.accent).fg(palette.on_accent),
            placeholder_style: Style::new().fg(palette.placeholder),
            style: Style::new().fg(palette.fg),
        }
    }
}

impl Default for TextAreaTheme {
    fn default() -> Self {
        Self::from_palette(&Palette::default())
    }
}

#[derive(Props, Default)]
pub struct TextAreaProps {
    /// 受控值(可含 `\n`)。
    pub value: String,
    /// 聚焦:渲染光标并响应键鼠。
    pub is_focus: bool,
    /// 内容变更回调。
    pub on_change: Handler<'static, String>,
    /// Enter 行为:`true` 插入换行(多行编辑);`false` 提交(`on_submit`)。
    pub multiline: bool,
    /// 单行模式(`multiline = false`)下 Enter 的提交回调。
    pub on_submit: Handler<'static, String>,
    /// 空值占位文案。
    pub placeholder: String,
    // 以下样式覆盖:`None` 用 `TextAreaTheme`,`Some(s)` 以 `theme.patch(s)` 覆盖。
    pub cursor_style: Option<Style>,
    pub placeholder_style: Option<Style>,
    pub style: Option<Style>,
}

/// 一个显示行:字符/字节区间(切片与光标换算用)与显示宽度。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WrapRow {
    /// 行首字符索引(含)。
    pub start_char: usize,
    /// 行尾字符索引(不含;软换行断在空白后,硬换行不含 `\n`)。
    pub end_char: usize,
    /// 行首字节偏移(含)。
    pub start_byte: usize,
    /// 行尾字节偏移(不含)。
    pub end_byte: usize,
    /// 显示宽度(终端列)。
    pub width: u16,
}

/// 按显示宽度做贪心软换行:空白后优先断行,无断点(超宽单词)则硬拆;
/// `\n` 结束当前行(不占显示宽度)。`width == 0` 视作 1,保证任何输入都有落点。
pub fn wrap_rows(value: &str, width: usize) -> Vec<WrapRow> {
    let max_width = width.max(1);
    let mut rows = Vec::new();
    let mut start_char = 0usize;
    let mut start_byte = 0usize;
    let mut col = 0usize;
    // 最近断点:断在某个空白字符**处**(该空白不进任何一行 —— 行宽不超过视口),
    // 记录 (空白字符索引, 空白字节, 不含该空白的行宽, 含该空白的累计宽)。
    // 空白自身永不触发换行(悬垂累计),供之后的溢出字符回溯断点。
    let mut last_break: Option<(usize, usize, usize, usize)> = None;

    for (char_index, (byte, ch)) in value.char_indices().enumerate() {
        if ch == '\n' {
            rows.push(WrapRow {
                start_char,
                end_char: char_index,
                start_byte,
                end_byte: byte,
                width: col as u16,
            });
            start_char = char_index + 1;
            start_byte = byte + 1;
            col = 0;
            last_break = None;
            continue;
        }
        let ch_width = ch.width().unwrap_or(0);
        if !ch.is_whitespace() && col + ch_width > max_width && col > 0 {
            match last_break {
                // 断点有效(断出的行非空):行尾落在空白处(空白不显示),空白之后的内容滚入新行。
                Some((break_char, break_byte, width_excl, width_incl))
                    if break_char > start_char =>
                {
                    rows.push(WrapRow {
                        start_char,
                        end_char: break_char,
                        start_byte,
                        end_byte: break_byte,
                        width: width_excl as u16,
                    });
                    let after_byte =
                        break_byte + value[break_byte..].chars().next().map_or(0, char::len_utf8);
                    start_char = break_char + 1;
                    start_byte = after_byte;
                    col -= width_incl;
                }
                // 无断点(超宽单词)或断出行为空:硬拆在当前字符之前。
                _ => {
                    rows.push(WrapRow {
                        start_char,
                        end_char: char_index,
                        start_byte,
                        end_byte: byte,
                        width: col as u16,
                    });
                    col = 0;
                    start_char = char_index;
                    start_byte = byte;
                }
            }
            last_break = None;
        }
        if ch.is_whitespace() {
            last_break = Some((char_index, byte, col, col + ch_width));
        }
        col += ch_width;
    }

    rows.push(WrapRow {
        start_char,
        end_char: value.chars().count(),
        start_byte,
        end_byte: value.len(),
        width: col as u16,
    });
    rows
}

/// 光标字符索引 → (显示行下标, 行内列)。软换行边界上的光标显示在**上一行行尾**
/// (输入位置语义);硬换行后的光标落下一行行首。
pub fn cursor_position(value: &str, rows: &[WrapRow], cursor: usize) -> Option<(usize, u16)> {
    // 行内区间 [start, end):光标严格落在区间内 → 该行。
    if let Some((index, row)) = rows
        .iter()
        .enumerate()
        .find(|(_, row)| row.start_char <= cursor && cursor < row.end_char)
    {
        return Some((index, width_before(value, row, cursor)));
    }
    // 边界光标(== 某行行尾):软换行处下一行行首也是它 → 归下一行行首(在那儿打字
    // 落到下一行);否则(硬换行前、或全文末尾)留在本行行尾。
    let (index, row) = rows
        .iter()
        .enumerate()
        .find(|(_, row)| row.end_char == cursor)?;
    match rows.get(index + 1) {
        Some(next) if next.start_char == cursor => Some((index + 1, 0)),
        _ => Some((index, width_before(value, row, cursor))),
    }
}

/// 行内 cursor 之前字符的总显示宽度。
fn width_before(value: &str, row: &WrapRow, cursor: usize) -> u16 {
    let mut width = 0usize;
    for (seen, ch) in (row.start_char..).zip(value[row.start_byte..row.end_byte].chars()) {
        if seen >= cursor {
            break;
        }
        width += ch.width().unwrap_or(0);
    }
    width as u16
}

/// (显示行下标, 行内列) → 光标字符索引:列落在哪个字符内,光标停在它之前
/// (与 Input 的点击定位语义一致);越界停在该行行尾。
pub fn cursor_index(value: &str, rows: &[WrapRow], row_index: usize, col: u16) -> Option<usize> {
    let row = rows.get(row_index)?;
    let mut width = 0usize;
    for (seen, ch) in (row.start_char..).zip(value[row.start_byte..row.end_byte].chars()) {
        let ch_width = ch.width().unwrap_or(0);
        if (col as usize) < width + ch_width {
            return Some(seen);
        }
        width += ch_width;
    }
    Some(row.end_char)
}

/// 上一次渲染的换行布局 + 竖向滚动。键盘 Up/Down 与点击定位在事件期读取
/// (滞后一帧,与框架其它指针映射一致)。
#[derive(Debug, Default)]
struct TextAreaLayout {
    rows: Vec<WrapRow>,
    scroll: u16,
}

/// 布局共享句柄(经 `use_hook` 跨帧保活)。
struct TextAreaShared(Rc<RefCell<TextAreaLayout>>);

impl Hook for TextAreaShared {}

#[component]
pub fn TextArea(props: &mut TextAreaProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    // 主题解析:theme slot 铺底,props 的 Option<Style> 在上 patch(None → 用主题)。
    let theme = hooks.use_component_theme::<TextAreaTheme>();
    let cursor_style = resolve_style(theme.cursor_style, props.cursor_style);
    let placeholder_style = resolve_style(theme.placeholder_style, props.placeholder_style);
    let style = resolve_style(theme.style, props.style);

    let input = hooks.use_state(Input::default);
    let focus = props.is_focus;
    let multiline = props.multiline;

    // 外部值同步(与 SearchInput 同模式):内部编辑经 on_change 回灌后相等,不重置光标。
    let external = props.value.clone();
    hooks.use_effect(
        move || {
            if input.read().value() != external {
                *input.write() = Input::new(external);
            }
        },
        props.value.clone(),
    );

    let size = hooks.use_previous_size();
    let shared = hooks.use_hook(|| TextAreaShared(Rc::default())).0.clone();

    let mut on_change = props.on_change.take();
    let mut on_submit = props.on_submit.take();

    // 键盘:聚焦时编辑。导航只用方向键/Home/End(字母键留给打字);Ctrl/Alt 组合放行。
    hooks.use_event_handler(EventScope::Current, EventPriority::Normal, {
        let shared = Rc::clone(&shared);
        move |event| {
            if !focus {
                return EventResult::Ignored;
            }
            let Event::Key(key) = event else {
                return EventResult::Ignored;
            };
            if key.kind != KeyEventKind::Press
                || key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
            {
                return EventResult::Ignored;
            }
            let commit = |changed: Option<StateChanged>,
                          on_change: &mut Handler<'static, String>| {
                if let Some(changed) = changed
                    && changed.value
                {
                    on_change(input.read().value().to_string());
                }
            };
            // Up/Down/Home/End 走显示行布局,其余交给 tui-input。
            let line_request = {
                let value = input.read().value().to_string();
                let cursor = input.read().cursor();
                let layout = shared.borrow();
                let position = cursor_position(&value, &layout.rows, cursor);
                match (position, key.code) {
                    (None, _) => None,
                    (Some((row, col)), KeyCode::Up) if row > 0 => {
                        cursor_index(&value, &layout.rows, row - 1, col)
                    }
                    (Some((row, col)), KeyCode::Down) if row + 1 < layout.rows.len() => {
                        cursor_index(&value, &layout.rows, row + 1, col)
                    }
                    (Some((row, _)), KeyCode::Home) => Some(layout.rows[row].start_char),
                    (Some((row, _)), KeyCode::End) => Some(layout.rows[row].end_char),
                    _ => None,
                }
                .map(InputRequest::SetCursor)
            };
            let request = match line_request {
                Some(request) => request,
                None => match key.code {
                    KeyCode::Enter | KeyCode::Char('\n') => {
                        if multiline {
                            InputRequest::InsertChar('\n')
                        } else {
                            on_submit(input.read().value().to_string());
                            return EventResult::Consumed;
                        }
                    }
                    KeyCode::Char(c) => InputRequest::InsertChar(c),
                    KeyCode::Backspace => InputRequest::DeletePrevChar,
                    KeyCode::Delete => InputRequest::DeleteNextChar,
                    KeyCode::Left => InputRequest::GoToPrevChar,
                    KeyCode::Right => InputRequest::GoToNextChar,
                    _ => return EventResult::Ignored,
                },
            };
            let changed = input.write().handle(request);
            commit(changed, &mut on_change);
            EventResult::Consumed
        }
    });

    // 鼠标:点击定位光标到 (显示行, 列)(聚焦时;与 Input 的 on_seek 语义一致)。
    if pointer_enabled(&hooks) {
        hooks.use_pointer(PointerOptions::default(), {
            let shared = Rc::clone(&shared);
            move |ev| {
                if !focus {
                    return EventResult::Ignored;
                }
                if let PointerKind::Click { .. } = ev.kind {
                    let value = input.read().value().to_string();
                    let layout = shared.borrow();
                    let row =
                        ev.position.y.saturating_sub(ev.area.y) as usize + layout.scroll as usize;
                    let col = ev.position.x.saturating_sub(ev.area.x);
                    if let Some(index) = cursor_index(&value, &layout.rows, row, col) {
                        input.write().handle(InputRequest::SetCursor(index));
                        EventResult::Consumed
                    } else {
                        EventResult::Ignored
                    }
                } else {
                    EventResult::Ignored
                }
            }
        });
    }

    // 渲染:换行布局 + 竖向滚动跟随光标。
    let value = input.read().value().to_string();
    let rows = wrap_rows(&value, size.width as usize);
    let cursor = input.read().cursor();
    let (cursor_row, cursor_col) = cursor_position(&value, &rows, cursor).unwrap_or((0, 0));

    let height = size.height as usize;
    let mut scroll = shared
        .borrow()
        .scroll
        .min(rows.len().saturating_sub(1) as u16);
    if cursor_row < scroll as usize {
        scroll = cursor_row as u16;
    } else if cursor_row + 1 > scroll as usize + height {
        scroll = (cursor_row + 1 - height) as u16;
    }
    {
        let mut layout = shared.borrow_mut();
        layout.rows = rows.clone();
        layout.scroll = scroll;
    }

    let visible = rows
        .iter()
        .skip(scroll as usize)
        .take(height)
        .map(|row| Line::from(value[row.start_byte..row.end_byte].to_string()));

    let cursor_y = size.y
        + (cursor_row as u16)
            .saturating_sub(scroll)
            .min(size.height.saturating_sub(1));
    let cursor_x = size.x + cursor_col.min(size.width.saturating_sub(1));

    element!(Fragment {
        if value.is_empty() {
            Text(
                text: props.placeholder.clone(),
                style: placeholder_style,
            )
        } else {
            Text(
                text: ratatui::text::Text::from(
                    visible.collect::<Vec<Line<'static>>>(),
                ),
                style: style,
            )
        }
        if focus {
            Positioned(x: cursor_x, y: cursor_y, width: 1u16, height: 1u16) {
                widget(Block::new().style(cursor_style))
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows_of(value: &str, width: usize) -> Vec<(usize, usize)> {
        wrap_rows(value, width)
            .iter()
            .map(|row| (row.start_char, row.end_char))
            .collect()
    }

    /// 空值有且仅有一个空行(占位与光标都有落点)。
    #[test]
    fn empty_value_yields_one_empty_row() {
        assert_eq!(rows_of("", 10), vec![(0, 0)]);
    }

    /// 未超宽时不换行;`\n` 硬断行且自身不占宽度。
    #[test]
    fn hard_breaks_follow_newlines() {
        assert_eq!(rows_of("ab\ncd", 10), vec![(0, 2), (3, 5)]);
    }

    /// 软换行断在空白之后,空白留在上一行行尾。
    #[test]
    fn soft_break_goes_after_whitespace() {
        // 宽 5:"hello" 恰好占满,空格+world 放不下 → 断在空格后。
        let rows = wrap_rows("hello world", 5);
        assert_eq!(rows.len(), 2);
        assert_eq!(
            &"hello world"[rows[0].start_byte..rows[0].end_byte],
            "hello"
        );
        assert_eq!(
            &"hello world"[rows[1].start_byte..rows[1].end_byte],
            "world"
        );
    }

    /// 无断点的超宽单词硬拆。
    #[test]
    fn overlong_word_breaks_hard() {
        let rows = wrap_rows("abcdefghij", 4);
        assert_eq!(
            rows.iter()
                .map(|row| &"abcdefghij"[row.start_byte..row.end_byte])
                .collect::<Vec<_>>(),
            vec!["abcd", "efgh", "ij"]
        );
    }

    /// 宽字符按显示列计算(中文占 2 列)。
    #[test]
    fn wide_chars_count_display_columns() {
        let rows = wrap_rows("中文字", 4);
        assert_eq!(
            rows.iter()
                .map(|row| &"中文字"[row.start_byte..row.end_byte])
                .collect::<Vec<_>>(),
            vec!["中文", "字"]
        );
    }

    /// 软换行边界的光标显示在上一行行尾;硬换行后落下一行行首。
    #[test]
    fn cursor_at_soft_break_shows_end_of_previous_row() {
        let value = "hello world";
        let rows = wrap_rows(value, 5);
        // cursor 5 = 软断点:上一行行尾。
        assert_eq!(cursor_position(value, &rows, 5), Some((0, 5)));
        // cursor 6 = 'w' 之前:下一行行首。
        assert_eq!(cursor_position(value, &rows, 6), Some((1, 0)));
        // 行尾之后(值末尾)落最后一行行尾。
        assert_eq!(cursor_position(value, &rows, 11), Some((1, 5)));

        let value = "ab\ncd";
        let rows = wrap_rows(value, 10);
        // cursor 2 = '\n' 之前 → 上一行行尾;cursor 3 = 'c' 之前 → 下一行行首。
        assert_eq!(cursor_position(value, &rows, 2), Some((0, 2)));
        assert_eq!(cursor_position(value, &rows, 3), Some((1, 0)));
    }

    /// 点击列 → 光标停在覆盖该列的字符之前,越界停行尾(与 Input 语义一致)。
    #[test]
    fn cursor_index_click_semantics() {
        let value = "hello world";
        let rows = wrap_rows(value, 5);
        assert_eq!(cursor_index(value, &rows, 0, 0), Some(0));
        assert_eq!(cursor_index(value, &rows, 1, 2), Some(8));
        assert_eq!(cursor_index(value, &rows, 1, 99), Some(11));
        assert_eq!(cursor_index(value, &rows, 9, 0), None, "行越界返回 None");
    }
}

#[cfg(test)]
mod interaction_tests {
    use super::*;
    use crate::mouse_test::Mounted;
    use crate::prelude::View;
    use crossterm::event::{KeyCode, MouseButton, MouseEventKind};
    use ratatui::layout::Constraint;
    use std::sync::{Arc, Mutex};

    type Log = Arc<Mutex<Vec<String>>>;

    #[derive(Clone, Default, Props)]
    struct HarnessProps {
        log: Log,
    }

    // 6 列宽的输入区:on_change 全程记录,末元素即当前值。
    #[component]
    fn Harness(props: &HarnessProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let value = hooks.use_state(String::new);
        let log = props.log.clone();
        let mut value_for_change = value;
        crate::element!(View(
            width: Constraint::Length(6),
            height: Constraint::Length(4)
        ) {
            TextArea(
                value: value.read().to_string(),
                is_focus: true,
                multiline: true,
                on_change: move |next: String| {
                    value_for_change.set(next.clone());
                    log.lock().unwrap().push(next);
                },
            )
        })
    }

    fn mount(log: &Log) -> Mounted {
        Mounted::mount(crate::element!(Harness(log: log.clone())), true)
    }

    fn type_chars(m: &mut Mounted, text: &str) {
        for ch in text.chars() {
            m.dispatch_key(KeyCode::Char(ch));
        }
    }

    /// 软换行后的 Up/Down 保持列宽:第二行行尾按 Up 回第一行行尾,插入落在第一行末。
    #[test]
    fn up_down_move_across_wrapped_rows() {
        let log: Log = Default::default();
        let mut m = mount(&log);

        type_chars(&mut m, "aaaa bbbb"); // 6 列 → "aaaa" / "bbbb"
        assert_eq!(log.lock().unwrap().last().unwrap(), "aaaa bbbb");

        m.dispatch_key(KeyCode::Up); // 行尾(bbbb 末)→ 上一行同列(行尾)
        type_chars(&mut m, "X");
        assert_eq!(log.lock().unwrap().last().unwrap(), "aaaaX bbbb");

        m.dispatch_key(KeyCode::Down); // 回到第二行
        type_chars(&mut m, "Y");
        assert_eq!(log.lock().unwrap().last().unwrap(), "aaaaX bbbbY");
    }

    /// multiline 下 Enter 插入换行;Home/End 落显示行首/尾。
    #[test]
    fn enter_inserts_newline_and_home_end_work() {
        let log: Log = Default::default();
        let mut m = mount(&log);

        type_chars(&mut m, "ab");
        m.dispatch_key(KeyCode::Enter);
        type_chars(&mut m, "cd");
        assert_eq!(log.lock().unwrap().last().unwrap(), "ab\ncd");

        m.dispatch_key(KeyCode::End); // 第二行行尾(已在)
        m.dispatch_key(KeyCode::Up); // 第一行行尾
        m.dispatch_key(KeyCode::Home); // 第一行行首
        type_chars(&mut m, "Z");
        assert_eq!(log.lock().unwrap().last().unwrap(), "Zab\ncd");
    }

    /// 点击定位光标到 (显示行, 列),随后输入从该处插入。
    #[test]
    fn clicking_positions_cursor() {
        let log: Log = Default::default();
        let mut m = mount(&log);

        type_chars(&mut m, "aaaa bbbb"); // 行 0 = "aaaa",行 1 = "bbbb"
        m.dispatch(MouseEventKind::Down(MouseButton::Left), 2, 1); // 行 1 列 2 → 停在该列字符前(char 7)
        m.dispatch(MouseEventKind::Up(MouseButton::Left), 2, 1);
        type_chars(&mut m, "Q");
        assert_eq!(log.lock().unwrap().last().unwrap(), "aaaa bbQbb");
    }
    #[test]
    fn cursor_highlight_preserves_the_character_under_it() {
        let log: Log = Default::default();
        let mut m = mount(&log);
        type_chars(&mut m, "abcd");
        m.dispatch_key(KeyCode::Home);
        assert_eq!(m.buffer()[(0, 0)].symbol(), "a");
        m.dispatch_key(KeyCode::End);
        type_chars(&mut m, "ef");
        assert_eq!(m.buffer()[(5, 0)].symbol(), "f");
    }
}
