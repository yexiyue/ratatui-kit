// 组件渲染测试 harness：把一个元素的组件树**单次离屏渲染**到 ratatui `TestBackend` 的
// `Buffer`，用于断言组件输出。
//
// 关键：`update` 经对象安全的 `&mut dyn UpdaterTerminal` 驱动，故可用 no-op 终端（无真实
// TTY）跑 update；draw 则用 `ratatui::Terminal<TestBackend>` 取 `Frame`。只覆盖一次渲染的
// 静态输出，不轮询 future/事件。

use crate::{
    AnyElement, ComponentDrawer, ElementRepr, render::tree::Tree, terminal::UpdaterTerminal,
};
use ratatui::buffer::Buffer;
use std::io;

// no-op 终端：`insert_before` 空操作。仅供驱动 update。事件不再经终端订阅（改由 `InputRuntime`)，
// harness 跑 `update_once`(含 `begin_frame`)建注册表但永不 `dispatch`。
struct NoopTerminal;

impl UpdaterTerminal for NoopTerminal {
    fn insert_before(
        &mut self,
        _height: u16,
        _draw_fn: Box<dyn FnOnce(&mut Buffer)>,
    ) -> io::Result<()> {
        Ok(())
    }
}

// 单次离屏渲染：建树 → no-op 跑 update → `TestBackend` 跑 draw → 返回 `Buffer` 克隆。
fn render_to_buffer(el: impl Into<AnyElement<'static>>, width: u16, height: u16) -> Buffer {
    render_to_buffer_frames(el, width, height, 1)
}

// 多帧离屏渲染：用于依赖上一帧布局信息的组件，例如 `Input::use_previous_size`。
fn render_to_buffer_frames(
    el: impl Into<AnyElement<'static>>,
    width: u16,
    height: u16,
    frames: usize,
) -> Buffer {
    let mut el = el.into();
    let helper = el.helper();
    let mut tree = Tree::new(el.props_mut(), helper);

    let mut noop = NoopTerminal;
    let mut terminal =
        ratatui::Terminal::new(ratatui::backend::TestBackend::new(width, height)).unwrap();

    for _ in 0..frames.max(1) {
        tree.update_once(&mut noop);
        terminal
            .draw(|frame| {
                let area = frame.area();
                let mut drawer = ComponentDrawer::new(frame, area);
                tree.draw_root(&mut drawer);
            })
            .unwrap();
    }

    terminal.backend().buffer().clone()
}

// 把 Buffer 第 `y` 行拼成字符串，便于断言。
fn row(buf: &Buffer, y: u16) -> String {
    (0..buf.area.width).map(|x| buf[(x, y)].symbol()).collect()
}

// 在整个 Buffer 里找某字符的首个位置（列, 行）。
fn find(buf: &Buffer, ch: &str) -> Option<(u16, u16)> {
    (0..buf.area.height)
        .flat_map(|y| (0..buf.area.width).map(move |x| (x, y)))
        .find(|&(x, y)| buf[(x, y)].symbol() == ch)
}

// 读取 Buffer 中 (x, y) 单元格的合成 Style（fg/bg/modifier），用于主题相关断言。
// 注意:渲染后 cell 恒为具体色,断言的是最终合成结果,而非中间 Style 的 None/Some。
fn cell_style(buf: &Buffer, x: u16, y: u16) -> ratatui::style::Style {
    buf[(x, y)].style()
}

#[test]
fn text_renders_content() {
    use crate::components::Text;
    let buf = render_to_buffer(crate::element!(Text(text: "hi")), 6, 1);
    assert!(row(&buf, 0).starts_with("hi"), "实际: {:?}", row(&buf, 0));
}

#[test]
fn wrapped_text_renders_hard_wrapped_lines() {
    use crate::components::WrappedText;
    let buf = render_to_buffer(
        crate::element!(WrappedText(
            text: "alpha beta gamma",
            wrap_width: 5,
        )),
        5,
        3,
    );

    assert!(
        row(&buf, 0).starts_with("alpha"),
        "第 1 行应渲染 alpha, 实际: {:?}",
        row(&buf, 0)
    );
    assert!(
        row(&buf, 1).starts_with("beta"),
        "第 2 行应渲染 beta, 实际: {:?}",
        row(&buf, 1)
    );
    assert!(
        row(&buf, 2).starts_with("gamma"),
        "第 3 行应渲染 gamma, 实际: {:?}",
        row(&buf, 2)
    );
}

#[test]
fn border_draws_box_around_child() {
    use crate::components::{Border, Text};
    let buf = render_to_buffer(crate::element!(Border { Text(text: "x") }), 5, 3);
    // 左上角为边框字符（非空格）。
    assert_ne!(buf[(0, 0)].symbol(), " ", "左上角应是边框字符");
    // 内容落在边框内（第 1 行）。
    assert!(
        row(&buf, 1).contains('x'),
        "内容应在边框内: {:?}",
        row(&buf, 1)
    );
}

#[test]
fn view_renders_children() {
    use crate::components::{Text, View};
    let buf = render_to_buffer(crate::element!(View { Text(text: "ab") }), 6, 1);
    assert!(row(&buf, 0).contains("ab"), "实际: {:?}", row(&buf, 0));
}

#[test]
fn hidden_cursor_input_starts_from_value_prefix() {
    use crate::components::{Border, Input};
    use ratatui::layout::Constraint;

    let buf = render_to_buffer_frames(
        crate::element!(Border(width: Constraint::Length(8), height: Constraint::Length(3)) {
            Input(
                input: tui_input::Input::new("dep".to_string()),
                hide_cursor: true,
            )
        }),
        8,
        3,
        2,
    );

    assert!(
        row(&buf, 1).contains("dep"),
        "隐藏光标的输入框应从值开头展示, 实际: {:?}",
        row(&buf, 1)
    );
}

#[test]
fn center_offsets_content_from_origin() {
    use crate::components::{Center, Text};
    use ratatui::layout::Constraint;
    // Center 需显式尺寸界定居中区域。
    let buf = render_to_buffer(
        crate::element!(Center(width: Constraint::Length(1), height: Constraint::Length(1)) {
            Text(text: "x")
        }),
        9,
        3,
    );
    let (x, _y) = find(&buf, "x").expect("应渲染 x");
    // 居中 → 不在左上原点列。
    assert!(x > 0, "居中后 x 应不在第 0 列, 实际列 {x}");
}

#[test]
fn handwritten_component_can_use_terminal_size_without_context_upgrade() {
    use crate::{Component, ComponentUpdater, Hooks, NoProps, UseTerminalSize};

    struct ManualSize;

    impl Component for ManualSize {
        type Props<'a> = NoProps;

        fn new(_props: &Self::Props<'_>) -> Self {
            Self
        }

        fn update(
            &mut self,
            _props: &mut Self::Props<'_>,
            mut hooks: Hooks,
            _updater: &mut ComponentUpdater,
        ) {
            let _ = hooks.use_terminal_size();
        }
    }

    let _ = render_to_buffer(crate::element!(ManualSize), 4, 1);
}

// 主题系统渲染断言:无 Provider 兜底、Palette 派生、组件级 override、Option<Style> per-call 覆盖。
// 用 Border 作参考组件(其左上角 (0,0) 边框单元格前景 = 解析后的 border_style.fg)。
mod theme_tests {
    // 刻意用「Default + 改字段」构造 Palette/BorderTheme:这正是 `#[non_exhaustive]` 下
    // 外部用户唯一可用的构造方式(禁结构体字面量),测试如实反映该写法。
    #![allow(clippy::field_reassign_with_default)]
    use super::{cell_style, find, render_to_buffer, render_to_buffer_frames};
    use crate::prelude::*;
    use ratatui::{
        buffer::Buffer,
        layout::Constraint,
        style::{Color, Modifier, Style},
        text::Line,
    };

    fn corner_fg(buf: &Buffer) -> Color {
        cell_style(buf, 0, 0).fg.unwrap_or(Color::Reset)
    }

    fn cell_fg(buf: &Buffer, x: u16, y: u16) -> Color {
        cell_style(buf, x, y).fg.unwrap_or(Color::Reset)
    }

    #[test]
    fn border_uses_default_theme_without_provider() {
        // 无 Provider:回退 BorderTheme::default() == from_palette(默认) → border = DarkGray。
        let buf = render_to_buffer(crate::element!(Border { Text(text: "x") }), 6, 3);
        assert_eq!(
            corner_fg(&buf),
            Color::DarkGray,
            "无 Provider 应回退默认主题边框色"
        );
    }

    #[test]
    fn palette_provider_drives_border_color() {
        let mut palette = Palette::default();
        palette.border = Color::Red;
        let buf = render_to_buffer(
            crate::element!(PaletteProvider(palette: palette) {
                Border { Text(text: "x") }
            }),
            6,
            3,
        );
        assert_eq!(corner_fg(&buf), Color::Red, "边框色应随注入的 Palette 派生");
    }

    #[test]
    fn theme_override_beats_palette() {
        let mut palette = Palette::default();
        palette.border = Color::Red;
        let mut override_theme = BorderTheme::default();
        override_theme.border_style = Style::new().fg(Color::Green);
        let buf = render_to_buffer(
            crate::element!(PaletteProvider(palette: palette) {
                ThemeOverride::<BorderTheme>(theme: override_theme) {
                    Border { Text(text: "x") }
                }
            }),
            6,
            3,
        );
        assert_eq!(
            corner_fg(&buf),
            Color::Green,
            "显式 override context 应优先于 palette 派生"
        );
    }

    #[test]
    fn per_call_option_style_overrides_theme() {
        let buf = render_to_buffer(
            crate::element!(Border(border_style: Some(Style::new().fg(Color::Magenta))) {
                Text(text: "x")
            }),
            6,
            3,
        );
        assert_eq!(
            corner_fg(&buf),
            Color::Magenta,
            "per-call Some(Style) 应 patch 覆盖主题"
        );
    }

    #[test]
    fn per_call_reset_clears_theme() {
        // Some(Style::reset()) → theme.patch(reset) 清空到终端默认(fg = Color::Reset)。
        let buf = render_to_buffer(
            crate::element!(Border(border_style: Some(Style::reset())) {
                Text(text: "x")
            }),
            6,
            3,
        );
        assert_eq!(
            corner_fg(&buf),
            Color::Reset,
            "Some(Style::reset()) 应清空主题到终端默认"
        );
    }

    // —— Text ——

    #[test]
    fn text_palette_drives_fg() {
        let mut palette = Palette::default();
        palette.fg = Color::Red;
        let buf = render_to_buffer(
            crate::element!(PaletteProvider(palette: palette) {
                Text(text: "x")
            }),
            6,
            1,
        );
        assert_eq!(
            cell_fg(&buf, 0, 0),
            Color::Red,
            "文本色应随 palette.fg 派生"
        );
    }

    #[test]
    fn text_per_call_style_overrides_theme() {
        let mut palette = Palette::default();
        palette.fg = Color::Red;
        let buf = render_to_buffer(
            crate::element!(PaletteProvider(palette: palette) {
                Text(text: "x", style: Some(Style::new().fg(Color::Magenta)))
            }),
            6,
            1,
        );
        assert_eq!(
            cell_fg(&buf, 0, 0),
            Color::Magenta,
            "per-call Some(Style) 应 patch 覆盖 Text 主题"
        );
    }

    // —— Input ——

    // Input 依赖 use_previous_size,需两帧;放进定尺 Border,内容区从 (1,1) 起。
    #[test]
    fn input_value_uses_palette_fg() {
        let mut palette = Palette::default();
        palette.fg = Color::Red;
        let buf = render_to_buffer_frames(
            crate::element!(PaletteProvider(palette: palette) {
                Border(width: Constraint::Length(8), height: Constraint::Length(3)) {
                    Input(input: tui_input::Input::new("ab".to_string()), hide_cursor: true)
                }
            }),
            8,
            3,
            2,
        );
        assert_eq!(
            cell_fg(&buf, 1, 1),
            Color::Red,
            "输入文本色应随 palette.fg 派生"
        );
    }

    #[test]
    fn input_placeholder_uses_palette_placeholder() {
        let mut palette = Palette::default();
        palette.placeholder = Color::Blue;
        let buf = render_to_buffer_frames(
            crate::element!(PaletteProvider(palette: palette) {
                Border(width: Constraint::Length(8), height: Constraint::Length(3)) {
                    Input(placeholder: "hi".to_string(), hide_cursor: true)
                }
            }),
            8,
            3,
            2,
        );
        assert_eq!(
            cell_fg(&buf, 1, 1),
            Color::Blue,
            "占位符色应随 palette.placeholder 派生"
        );
    }

    #[test]
    fn input_cursor_uses_palette_accent_by_default() {
        // 默认 palette:accent = Cyan;空值 + 显示光标 → (1,1) 为光标块,底色 = accent。
        let buf = render_to_buffer_frames(
            crate::element!(Border(width: Constraint::Length(8), height: Constraint::Length(3)) {
                Input(input: tui_input::Input::default())
            }),
            8,
            3,
            2,
        );
        assert_eq!(
            cell_style(&buf, 1, 1).bg.unwrap_or(Color::Reset),
            Color::Cyan,
            "默认主题下光标块底色应为 accent"
        );
    }

    // —— Modal ——

    #[test]
    fn modal_mask_dims_backdrop_by_default() {
        // 默认 ModalTheme 遮罩带 DIM;弹窗内容居中,角落 (0,0) 属遮罩区。
        let buf = render_to_buffer(
            crate::element!(Modal(
                open: true,
                width: Constraint::Length(4),
                height: Constraint::Length(1),
            ) {
                Text(text: "m")
            }),
            10,
            5,
        );
        assert!(
            cell_style(&buf, 0, 0).add_modifier.contains(Modifier::DIM),
            "默认 Modal 遮罩应使背景变暗(DIM)"
        );
    }

    #[test]
    fn modal_mask_per_call_style_patches_theme() {
        // per-call bg 叠加在主题 DIM 之上:遮罩既有底色又保留 DIM。
        let buf = render_to_buffer(
            crate::element!(Modal(
                open: true,
                width: Constraint::Length(4),
                height: Constraint::Length(1),
                style: Some(Style::new().bg(Color::Red)),
            ) {
                Text(text: "m")
            }),
            10,
            5,
        );
        let s = cell_style(&buf, 0, 0);
        assert_eq!(s.bg, Some(Color::Red), "per-call 底色应 patch 进遮罩");
        assert!(
            s.add_modifier.contains(Modifier::DIM),
            "per-call patch 应保留主题 DIM"
        );
    }

    #[test]
    fn modal_mask_reset_clears_dim() {
        // Some(Style::reset()) → theme.patch(reset) 清空 DIM。
        let buf = render_to_buffer(
            crate::element!(Modal(
                open: true,
                width: Constraint::Length(4),
                height: Constraint::Length(1),
                style: Some(Style::reset()),
            ) {
                Text(text: "m")
            }),
            10,
            5,
        );
        assert!(
            !cell_style(&buf, 0, 0).add_modifier.contains(Modifier::DIM),
            "Some(Style::reset()) 应清空遮罩 DIM"
        );
    }

    // —— SearchInput ——
    // 注:激活/成功/失败态由内部 editing 状态驱动,harness 不派发事件,只能测非激活基态;
    // 状态色的 patch 逻辑与已验证组件同构。

    #[test]
    fn search_input_default_border_from_palette() {
        let mut palette = Palette::default();
        palette.border = Color::Red;
        let buf = render_to_buffer_frames(
            crate::element!(PaletteProvider(palette: palette) {
                SearchInput(placeholder: "q".to_string())
            }),
            12,
            3,
            2,
        );
        assert_eq!(corner_fg(&buf), Color::Red, "非激活边框应取 palette.border");
    }

    #[test]
    fn search_input_placeholder_from_palette() {
        let mut palette = Palette::default();
        palette.placeholder = Color::Blue;
        let buf = render_to_buffer_frames(
            crate::element!(PaletteProvider(palette: palette) {
                SearchInput(placeholder: "hi".to_string())
            }),
            12,
            3,
            2,
        );
        assert_eq!(
            cell_fg(&buf, 1, 1),
            Color::Blue,
            "占位符应取 palette.placeholder"
        );
    }

    // —— ConfirmModal ——

    #[test]
    fn confirm_modal_selected_button_is_accent_bold() {
        // confirm_selected 默认 false → 取消按钮(第一个)默认选中;title/confirm_text 均设为无 'C',
        // 使 'C' 唯一落在选中的 "Cancel" 上。
        let buf = render_to_buffer(
            crate::element!(ConfirmModal(
                open: true,
                title: Line::from("Prompt"),
                confirm_text: "OK".to_string(),
                cancel_text: "Cancel".to_string(),
            )),
            50,
            16,
        );
        let (x, y) = find(&buf, "C").expect("应渲染选中按钮 Cancel 的 'C'");
        let s = cell_style(&buf, x, y);
        assert!(
            s.add_modifier.contains(Modifier::BOLD),
            "选中按钮标签应为 BOLD"
        );
        assert_eq!(
            s.fg,
            Some(Color::Cyan),
            "选中按钮标签前景应取默认主题 accent(Cyan)"
        );
    }

    // —— Select / MultiSelect ——

    #[test]
    fn select_highlight_uses_palette_selection() {
        let mut palette = Palette::default();
        palette.selection = Color::Magenta;
        let buf = render_to_buffer_frames(
            crate::element!(PaletteProvider(palette: palette) {
                Select<String>(items: vec!["a".to_string(), "b".to_string()], default_index: 0)
            }),
            10,
            5,
            2,
        );
        let (x, y) = find(&buf, "a").expect("应渲染首个列表项 'a'");
        assert_eq!(
            cell_style(&buf, x, y).bg,
            Some(Color::Magenta),
            "选中行底色应取 palette.selection"
        );
    }

    // —— TreeSelect(中性组件默认选中可见)——

    #[test]
    fn tree_select_default_selection_is_visible() {
        // 默认 TreeSelectTheme 提供可见高亮(bg = selection = 默认 Cyan);
        // 迁移前默认 highlight 为空、选中不可见。
        let items = vec![tui_tree_widget::TreeItem::new_leaf("a", "Alpha")];
        let buf = render_to_buffer_frames(
            crate::element!(TreeSelect<&'static str>(
                items: items,
                default_selection: vec!["a"],
            )),
            12,
            3,
            2,
        );
        let (x, y) = find(&buf, "A").expect("应渲染节点文本 Alpha");
        assert_eq!(
            cell_style(&buf, x, y).bg,
            Some(Color::Cyan),
            "默认主题下 TreeSelect 选中行应可见(bg = selection)"
        );
    }

    // —— Table ——

    #[test]
    fn table_header_border_and_selection_from_palette() {
        use std::sync::Arc;
        let mut palette = Palette::default();
        palette.accent = Color::Magenta;
        palette.selection = Color::Green;
        palette.border = Color::Red;
        let render_row: RenderTableRow<String> =
            Arc::new(|row: &String, _selected: bool| vec![TableCell::new(row.clone())]);
        let buf = render_to_buffer_frames(
            crate::element!(PaletteProvider(palette: palette) {
                Table<String>(
                    columns: vec![TableColumn::new("H", Constraint::Length(5))],
                    rows: vec!["r".to_string()],
                    render_row: Some(render_row),
                    default_index: 0,
                )
            }),
            14,
            8,
            2,
        );
        let (hx, hy) = find(&buf, "H").expect("应渲染表头 'H'");
        assert_eq!(
            cell_style(&buf, hx, hy).fg,
            Some(Color::Magenta),
            "表头前景应取 palette.accent"
        );
        let (rx, ry) = find(&buf, "r").expect("应渲染数据行 'r'");
        assert_eq!(
            cell_style(&buf, rx, ry).bg,
            Some(Color::Green),
            "选中行底色应取 palette.selection"
        );
        // 外框(Outer 模式)左上角取 palette.border。
        assert_eq!(
            cell_style(&buf, 0, 0).fg,
            Some(Color::Red),
            "表格外框应取 palette.border"
        );
    }
}

// 运行时换肤:写 `Atom<Palette>` → 订阅它的 `PaletteProvider` 在下一帧重渲并换色。
// 手动驱动两帧(帧间写入),而非 render_to_buffer_frames,以在中途改状态。
mod runtime_theme_tests {
    #![allow(clippy::field_reassign_with_default)]
    use super::{NoopTerminal, cell_style};
    use crate::render::tree::Tree;
    use crate::{Atom, ComponentDrawer, UseAtom, prelude::*};
    use ratatui::{backend::TestBackend, style::Color};
    use ratatui_kit_macros::component;

    static PROBE_PALETTE: Atom<Palette> = Atom::new(Palette::default);

    // 探针:订阅全局 `Atom<Palette>`,把当前值注入子树。每帧重跑 body → 重读 atom。
    #[component]
    fn ThemedProbe(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let palette = hooks.use_atom(&PROBE_PALETTE);
        element!(PaletteProvider(palette: palette.get()) {
            Border { Text(text: "x") }
        })
    }

    fn draw(tree: &mut Tree, terminal: &mut ratatui::Terminal<TestBackend>) {
        terminal
            .draw(|frame| {
                let area = frame.area();
                let mut drawer = ComponentDrawer::new(frame, area);
                tree.draw_root(&mut drawer);
            })
            .unwrap();
    }

    #[test]
    fn runtime_palette_switch_recolors_next_frame() {
        // 复位到默认(测试共享进程级全局 OWNER)。
        PROBE_PALETTE.set(Palette::default());

        let mut el: AnyElement<'static> = crate::element!(ThemedProbe).into();
        let helper = el.helper();
        let mut tree = Tree::new(el.props_mut(), helper);
        let mut noop = NoopTerminal;
        let mut terminal = ratatui::Terminal::new(TestBackend::new(6, 3)).unwrap();

        // 帧 1:默认 palette → 边框角落 DarkGray。
        tree.update_once(&mut noop);
        draw(&mut tree, &mut terminal);
        assert_eq!(
            cell_style(terminal.backend().buffer(), 0, 0).fg,
            Some(Color::DarkGray),
            "帧1 应为默认主题边框色"
        );

        // 运行时写入新 palette(模拟换肤)。
        let mut red = Palette::default();
        red.border = Color::Red;
        PROBE_PALETTE.set(red);

        // 帧 2:同一棵树重渲 → 边框随 Atom 写入变 Red。
        tree.update_once(&mut noop);
        draw(&mut tree, &mut terminal);
        assert_eq!(
            cell_style(terminal.backend().buffer(), 0, 0).fg,
            Some(Color::Red),
            "写入 Atom<Palette> 后下一帧应换色"
        );
    }
}

mod scroll_view_tests {
    use super::{render_to_buffer, row};
    use crate::prelude::*;
    use ratatui::{
        layout::{Constraint, Direction},
        widgets::Block,
    };

    #[test]
    fn large_fill_weight_does_not_panic() {
        let buf = render_to_buffer(
            element!(ScrollView(flex_direction: Direction::Horizontal) {
                View(width: Constraint::Fill(820)) {
                    Text(text: "wide")
                }
            }),
            20,
            3,
        );

        assert!(
            row(&buf, 0).contains("wide"),
            "应渲染内容, 实际: {:?}",
            row(&buf, 0)
        );
    }

    #[test]
    fn boundary_sized_content_remains_visible() {
        let buf = render_to_buffer(
            element!(ScrollView(flex_direction: Direction::Vertical) {
                View(height: Constraint::Length(2)) {
                    Text(text: "fit")
                }
            }),
            12,
            3,
        );

        assert!(
            row(&buf, 0).contains("fit"),
            "临界尺寸内容应可见, 实际: {:?}",
            row(&buf, 0)
        );
    }

    #[component]
    fn ControlledScroll(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let scroll_state = hooks.use_state(ScrollViewState::default);

        element!(ScrollView(
            state: scroll_state,
            block: Block::bordered(),
        ) {
            Text(text: "boxed")
        })
    }

    #[test]
    fn controlled_state_with_block_renders() {
        let buf = render_to_buffer(element!(ControlledScroll), 12, 3);

        assert_ne!(buf[(0, 0)].symbol(), " ", "应渲染边框");
        assert!(
            row(&buf, 1).contains("boxed"),
            "应渲染受控内容, 实际: {:?}",
            row(&buf, 1)
        );
    }

    // D1:内区收敛为 block.inner() 后,内容不再盖掉右边框。短内容(无滚动条)时
    // 左右边框都应完整保留(旧的 width-1 会让内容 blit 到右边框列上)。
    #[test]
    fn bordered_scrollview_preserves_both_side_borders() {
        let buf = render_to_buffer(
            element!(ScrollView(block: Block::bordered()) {
                Text(text: "hi")
            }),
            10,
            3,
        );

        assert_eq!(buf[(0, 1)].symbol(), "│", "左边框应保留");
        assert_eq!(buf[(9, 1)].symbol(), "│", "右边框不应被内容覆盖");
    }

    // D4:ScrollView 嵌套 ScrollView 不应 panic(共享 scroll buffer 栈的 save/restore)。
    #[component]
    fn NestedScroll(_hooks: Hooks) -> impl Into<AnyElement<'static>> {
        element!(ScrollView(flex_direction: Direction::Vertical) {
            ScrollView(flex_direction: Direction::Vertical) {
                Text(text: "deep")
            }
        })
    }

    #[test]
    fn nested_scrollview_does_not_panic() {
        let buf = render_to_buffer(element!(NestedScroll), 12, 4);
        assert!(
            row(&buf, 0).contains("deep"),
            "嵌套 ScrollView 应渲染内层内容, 实际: {:?}",
            row(&buf, 0)
        );
    }

    // D3:显示横向滚动条会占掉一行视口;偏移须按视口(而非原始区)裁剪,否则最后一行滚不到。
    #[component]
    fn TallScroll(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let scroll_state = hooks.use_state(|| {
            let mut state = ScrollViewState::default();
            state.scroll_to_bottom();
            state
        });

        element!(ScrollView(
            flex_direction: Direction::Vertical,
            state: scroll_state,
            scrollbars: Scrollbars {
                horizontal_scrollbar_visibility: ScrollbarVisibility::Always,
                ..Default::default()
            },
        ) {
            View(height: Constraint::Length(1)) { Text(text: "r0") }
            View(height: Constraint::Length(1)) { Text(text: "r1") }
            View(height: Constraint::Length(1)) { Text(text: "r2") }
            View(height: Constraint::Length(1)) { Text(text: "r3") }
            View(height: Constraint::Length(1)) { Text(text: "r4") }
        })
    }

    #[test]
    fn last_row_reachable_when_horizontal_scrollbar_shown() {
        // 视口高 3,底部横向滚动条占 1 行 → 内容视口 2 行;5 行内容滚到底后末行 r4 应可见。
        let buf = render_to_buffer(element!(TallScroll), 8, 3);
        let text = (0..3).map(|y| row(&buf, y)).collect::<Vec<_>>().join("|");
        assert!(text.contains("r4"), "末行应可滚到, 实际: {text:?}");
    }
}

// 路由渲染链路集成测试:验证 `RouterProvider` + `Outlet` 按 `index_path` 选中并
// 渲染正确组件,以及嵌套 `Outlet` 消费剩余 path。仅在 `router` 特性下编译。
#[cfg(feature = "router")]
mod router_tests {
    use super::{render_to_buffer, row};
    use crate::prelude::*;

    // 零状态测试页面,各渲染可辨识文本。
    #[component]
    fn HomePage(_hooks: Hooks) -> impl Into<AnyElement<'static>> {
        element!(Text(text: "HOME"))
    }

    #[component]
    fn AboutPage(_hooks: Hooks) -> impl Into<AnyElement<'static>> {
        element!(Text(text: "ABOUT"))
    }

    // 父布局:仅渲染 Outlet,用于验证嵌套消费剩余 path。
    #[component]
    fn LayoutPage(_hooks: Hooks) -> impl Into<AnyElement<'static>> {
        element!(Outlet)
    }

    fn two_pages() -> Routes {
        routes! {
            "/home" => HomePage,
            "/about" => AboutPage,
        }
        .into()
    }

    #[test]
    fn renders_index_route() {
        let buf = render_to_buffer(
            element!(RouterProvider(routes: two_pages(), index_path: "/home")),
            8,
            1,
        );
        assert!(row(&buf, 0).contains("HOME"), "实际: {:?}", row(&buf, 0));
    }

    #[test]
    fn renders_sibling_route_by_index_path() {
        let buf = render_to_buffer(
            element!(RouterProvider(routes: two_pages(), index_path: "/about")),
            8,
            1,
        );
        assert!(row(&buf, 0).contains("ABOUT"), "实际: {:?}", row(&buf, 0));
    }

    #[test]
    fn nested_outlet_consumes_remaining_path() {
        let routes: Routes = routes! {
            "/" => LayoutPage {
                "/home" => HomePage,
            },
        }
        .into();
        let buf = render_to_buffer(
            element!(RouterProvider(routes: routes, index_path: "/home")),
            8,
            1,
        );
        assert!(
            row(&buf, 0).contains("HOME"),
            "嵌套应渲染 HOME, 实际: {:?}",
            row(&buf, 0)
        );
    }
}
