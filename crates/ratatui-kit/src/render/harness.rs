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
            scroll_view_state: scroll_state,
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
