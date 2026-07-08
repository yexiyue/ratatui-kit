//! 主题系统示例。
//!
//! 演示五件事:
//! 1. **全局 palette**:`PaletteProvider` 注入一个 `Palette`,子树内组件自动据此上色。
//! 2. **运行时换肤**:`Palette` 存进全局 `Atom`,按 `t` 写入即触发整棵子树重渲换色。
//! 3. **组件级 override**:`ThemeOverride::<BorderTheme>` 只改这一类组件的主题(解析链第一级)。
//! 4. **per-call `Option<Style>` 覆盖**:`border_style: Some(s)` 以 `theme.patch(s)` 局部覆盖。
//! 5. **`Style::reset()` 清空**:`Some(Style::reset())` 把该 slot 清回终端默认。

use ratatui_kit::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    ratatui::{
        layout::{Constraint, Direction},
        style::{Color, Style},
        text::Line,
    },
};

// 全局响应式调色板:写入即唤醒订阅它的 `PaletteProvider` 重渲。
static PALETTE: Atom<Palette> = Atom::new(Palette::default);

// 三套预置调色板,按 `t` 循环切换。均以 `Palette::default()` 起手再改字段
// (`Palette` 是 `#[non_exhaustive]`,禁结构体字面量,只能这样构造)。
fn presets() -> [(&'static str, Palette); 3] {
    let default = Palette::default();

    let mut ocean = Palette::default();
    ocean.accent = Color::Rgb(94, 175, 255);
    ocean.selection = Color::Rgb(30, 70, 120);
    ocean.on_accent = Color::White;
    ocean.border = Color::Rgb(70, 100, 140);
    ocean.border_active = Color::Rgb(94, 175, 255);
    ocean.placeholder = Color::Rgb(90, 110, 140);

    let mut sunset = Palette::default();
    sunset.accent = Color::Rgb(255, 140, 90);
    sunset.selection = Color::Rgb(120, 60, 40);
    sunset.on_accent = Color::Black;
    sunset.border = Color::Rgb(150, 100, 80);
    sunset.border_active = Color::Rgb(255, 170, 120);
    sunset.warning = Color::Rgb(255, 200, 90);
    sunset.placeholder = Color::Rgb(150, 110, 90);

    [("default", default), ("ocean", ocean), ("sunset", sunset)]
}

#[tokio::main]
async fn main() {
    element!(ThemeDemo)
        .fullscreen()
        .await
        .expect("Failed to run the application");
}

#[component]
fn ThemeDemo(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let palette = hooks.use_atom(&PALETTE);
    let mut preset_index = hooks.use_state(|| 0usize);
    let mut exit = hooks.use_exit();

    hooks.use_event_handler(EventScope::Current, EventPriority::Normal, move |event| {
        let Event::Key(key) = event else {
            return EventResult::Ignored;
        };
        if key.kind != KeyEventKind::Press {
            return EventResult::Ignored;
        }
        match key.code {
            KeyCode::Char('t') | KeyCode::Char('T') => {
                let next = (preset_index.get() + 1) % presets().len();
                preset_index.set(next);
                // 运行时写入 → 订阅的 PaletteProvider 下一帧换色。
                PALETTE.set(presets()[next].1);
                EventResult::Consumed
            }
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                exit();
                EventResult::Consumed
            }
            _ => EventResult::Ignored,
        }
    });

    let preset_name = presets()[preset_index.get()].0;

    // ④ 组件级 override:构造一个绿色边框的 BorderTheme override。
    // 只影响被 `ThemeOverride::<BorderTheme>` 包裹的子树里的 Border。
    let mut green_border = BorderTheme::default();
    green_border.border_style = Style::new().green();

    let items = vec![
        "Select 高亮取自 palette".to_string(),
        "on_accent 前景 + selection 底".to_string(),
        "换肤时随之改变".to_string(),
    ];

    element!(PaletteProvider(palette: palette.get()) {
        Border(
            flex_direction: Direction::Vertical,
            gap: 1,
            top_title: Line::from(format!(" theme system · preset: {preset_name} ")).centered(),
            bottom_title: Line::from(" t 切换主题 · q quit · Ctrl+C exit ").centered(),
        ) {
            // ① 全局 palette:普通组件自动上色(边框取 palette.border)。
            Border(height: Constraint::Length(3), top_title: Line::from(" ① 全局 palette ")) {
                Text(text: "边框/文本颜色全部来自当前 Palette")
            }

            // ② per-call Option<Style>:局部覆盖成 magenta,不受主题影响。
            Border(
                height: Constraint::Length(3),
                border_style: Some(Style::new().magenta()),
                top_title: Line::from(" ② per-call Some(Style) → magenta "),
            ) {
                Text(text: "border_style: Some(Style::new().magenta())")
            }

            // ③ Style::reset():把边框清回终端默认色(不随主题)。
            Border(
                height: Constraint::Length(3),
                border_style: Some(Style::reset()),
                top_title: Line::from(" ③ Some(Style::reset()) → 终端默认 "),
            ) {
                Text(text: "border_style: Some(Style::reset())")
            }

            // ④ 组件级 override:仅这层内的 Border 用绿色主题。
            ThemeOverride::<BorderTheme>(theme: green_border) {
                Border(height: Constraint::Length(3), top_title: Line::from(" ④ ThemeOverride<BorderTheme> → green ")) {
                    Text(text: "只有本层的 Border 变绿;外层不受影响")
                }
            }

            // ⑤ 富主题组件:Select 高亮取自 palette(active:false,仅展示不抢键)。
            Select<String>(
                height: Constraint::Length(5),
                items: items,
                active: false,
                default_index: 0,
                top_title: Line::from(" ⑤ Select(高亮取自 palette) "),
            )
        }
    })
}
