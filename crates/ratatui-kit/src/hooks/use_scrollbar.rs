// use_scrollbar hook:自绘内容的滚动条「渲染 + 指针拖拽」一站式接入。
//
// 适用场景:内容绘制完全自定义、不适合用 `ScrollView` 整体包裹,但边缘仍需要
// 一条可交互滚动条(如阅读器正文、虚拟列表旁的指示条)。一次 `use_scrollbar`
// 调用同时获得 ratatui `Scrollbar` 渲染、抓滑块拖动、点滑轨空白跳转;
// 滑块数学与 `ScrollView` 同源(`AxisGeometry`,复刻 ratatui `part_lengths`),
// 滑轨两端精确映射到 0 / 最大偏移,与显示量化互为 ±1 格近似逆。

use crate::{
    Hooks, PointerOptions, UsePointer,
    components::scroll_view::{
        AxisGeometry, ScrollbarGeometry, ScrollbarInteraction, ScrollbarStyle,
    },
    hooks::use_pointer::pointer_enabled,
};
use ratatui::{
    layout::Rect,
    widgets::{ScrollbarOrientation, ScrollbarState},
};
use std::{cell::RefCell, rc::Rc};

mod private {
    pub trait Sealed {}
    impl Sealed for crate::hooks::Hooks<'_, '_> {}
}

/// 滑轨相对组件 area 的落位;滚动方向由落位决定(纵向/横向)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackPlacement {
    /// 纵向,贴右缘,上下各缩进 `n` 行。
    RightInward(u16),
    /// 纵向,贴左缘,上下各缩进 `n` 行。
    LeftInward(u16),
    /// 横向,贴底缘,左右各缩进 `n` 列。
    BottomInward(u16),
    /// 横向,贴顶缘,左右各缩进 `n` 列。
    TopInward(u16),
}

impl TrackPlacement {
    /// 纵向落位返回 true。
    pub fn is_vertical(&self) -> bool {
        matches!(self, Self::RightInward(_) | Self::LeftInward(_))
    }

    fn orientation(&self) -> ScrollbarOrientation {
        match self {
            Self::RightInward(_) => ScrollbarOrientation::VerticalRight,
            Self::LeftInward(_) => ScrollbarOrientation::VerticalLeft,
            Self::BottomInward(_) => ScrollbarOrientation::HorizontalBottom,
            Self::TopInward(_) => ScrollbarOrientation::HorizontalTop,
        }
    }
}

impl Default for TrackPlacement {
    fn default() -> Self {
        // 缩进 1 行:避开组件四角(与常见边框布局兼容)。
        Self::RightInward(1)
    }
}

/// `use_scrollbar` 的参数。单位与 ratatui `ScrollbarState` 对齐:
/// `position` 是内容偏移,可滚动范围是 `content_length - viewport_length`,
/// `viewport_length`(视口可见长度)决定滑块大小。
#[derive(Default)]
pub struct ScrollbarOptions {
    /// 滑轨落位(方向 + 贴边),默认右侧纵向、上下缩进 1 行。
    pub placement: TrackPlacement,
    /// 内容总长(`position` 的取值域为 `0..=content_length - viewport_length`)。
    pub content_length: usize,
    /// 视口可见长度(决定滑块占比)。
    pub viewport_length: usize,
    /// 当前内容偏移;`on_seek` 回传同一单位。
    pub position: usize,
    /// 滚动条外观;默认按落位使用 ratatui 符号。
    pub scrollbar: ScrollbarStyle<'static>,
}

/// 由组件 area 与落位推导滑轨矩形(交叉轴恒为 1 格宽)。
fn track_rect(area: Rect, placement: TrackPlacement) -> Rect {
    if area.is_empty() {
        return area;
    }
    match placement {
        TrackPlacement::RightInward(n) => Rect {
            x: area.right().saturating_sub(1),
            y: area.y.saturating_add(n),
            width: 1,
            height: area.height.saturating_sub(n.saturating_mul(2)),
        },
        TrackPlacement::LeftInward(n) => Rect {
            x: area.x,
            y: area.y.saturating_add(n),
            width: 1,
            height: area.height.saturating_sub(n.saturating_mul(2)),
        },
        TrackPlacement::BottomInward(n) => Rect {
            x: area.x.saturating_add(n),
            y: area.bottom().saturating_sub(1),
            width: area.width.saturating_sub(n.saturating_mul(2)),
            height: 1,
        },
        TrackPlacement::TopInward(n) => Rect {
            x: area.x.saturating_add(n),
            y: area.y,
            width: area.width.saturating_sub(n.saturating_mul(2)),
            height: 1,
        },
    }
}

/// 本帧滑块几何。`None` = 不可滚动(content ≤ viewport)或滑轨长度为 0。
fn axis_geometry(area: Rect, options: &ScrollbarOptions) -> Option<AxisGeometry> {
    let scrollable = options
        .content_length
        .saturating_sub(options.viewport_length);
    if scrollable == 0 {
        return None;
    }
    AxisGeometry::new(
        track_rect(area, options.placement),
        options.viewport_length,
        scrollable,
        options.position.min(scrollable),
        options.placement.is_vertical(),
        options.scrollbar.arrow_lengths(),
    )
}

/// hook 实例:渲染参数由 `use_scrollbar` 每帧写入,`pre_component_draw` 记录
/// 组件 area,`post_component_draw` 渲染滑块并发布最新几何给指针闭包。
struct ScrollbarHook {
    options: ScrollbarOptions,
    area: Rect,
    // 最新几何(None = 不可滚动);事件期读取滞后一帧,与框架其它指针映射一致。
    shared: Rc<RefCell<ScrollbarInteraction>>,
}

impl ScrollbarHook {
    fn new() -> Self {
        Self {
            options: ScrollbarOptions::default(),
            area: Rect::default(),
            shared: Rc::default(),
        }
    }
}

impl crate::hooks::Hook for ScrollbarHook {
    fn pre_component_draw(&mut self, drawer: &mut crate::ComponentDrawer<'_, '_>) {
        self.area = drawer.area;
    }

    fn post_component_draw(&mut self, drawer: &mut crate::ComponentDrawer<'_, '_>) {
        let Some(geometry) = axis_geometry(self.area, &self.options) else {
            self.shared.borrow_mut().geometry = ScrollbarGeometry::default();
            return;
        };
        let mut state = ScrollbarState::new(geometry.scrollable.saturating_add(1))
            .position(geometry.offset)
            .viewport_content_length(geometry.viewport);
        let scrollbar = self.options.scrollbar.widget_with_state(
            self.options.placement.orientation(),
            self.shared
                .borrow()
                .is_hovered(self.options.placement.is_vertical()),
            self.shared
                .borrow()
                .is_dragging(self.options.placement.is_vertical()),
        );
        drawer.render_stateful_widget(scrollbar, geometry.track, &mut state);
        self.shared.borrow_mut().geometry = if self.options.placement.is_vertical() {
            ScrollbarGeometry {
                vertical: Some(geometry),
                horizontal: None,
            }
        } else {
            ScrollbarGeometry {
                vertical: None,
                horizontal: Some(geometry),
            }
        };
    }
}

pub trait UseScrollbar: private::Sealed {
    /// 在组件 area 边缘渲染一条可拖拽滚动条:抓住滑块拖动 / 点滑轨空白跳转,
    /// 滑轨两端精确映射到 `0` / `content_length - viewport_length`。
    /// `on_seek` 回传目标内容偏移,调用方写回自己的滚动状态即可。
    ///
    /// 仅应用启用鼠标捕获时注册拖拽(`use_pointer` 未启用时 debug 断言)。
    fn use_scrollbar<F>(&mut self, options: ScrollbarOptions, on_seek: F)
    where
        F: FnMut(usize) + 'static;
}

impl UseScrollbar for Hooks<'_, '_> {
    fn use_scrollbar<F>(&mut self, options: ScrollbarOptions, on_seek: F)
    where
        F: FnMut(usize) + 'static,
    {
        let shared = {
            let hook = self.use_hook(ScrollbarHook::new);
            hook.options = options;
            Rc::clone(&hook.shared)
        };
        if pointer_enabled(self) {
            let mut on_seek = on_seek;
            self.use_pointer(
                PointerOptions {
                    track_move: true,
                    ..Default::default()
                },
                move |event| {
                    shared
                        .borrow_mut()
                        .handle(event, |target, _| on_seek(target))
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::layout::Rect;

    fn area() -> Rect {
        Rect::new(0, 0, 80, 24)
    }

    #[test]
    fn track_rect_places_all_four_edges() {
        let a = area();
        assert_eq!(
            track_rect(a, TrackPlacement::RightInward(1)),
            Rect::new(79, 1, 1, 22)
        );
        assert_eq!(
            track_rect(a, TrackPlacement::LeftInward(2)),
            Rect::new(0, 2, 1, 20)
        );
        assert_eq!(
            track_rect(a, TrackPlacement::BottomInward(1)),
            Rect::new(1, 23, 78, 1)
        );
        assert_eq!(
            track_rect(a, TrackPlacement::TopInward(3)),
            Rect::new(3, 0, 74, 1)
        );
    }

    /// 缩进超过半宽/半高时饱和为 0 长度,而不是下溢 panic。
    #[test]
    fn track_rect_saturates_oversized_inset() {
        assert_eq!(
            track_rect(area(), TrackPlacement::RightInward(20)),
            Rect::new(79, 20, 1, 0)
        );
    }

    #[test]
    fn axis_geometry_hidden_when_content_fits() {
        let options = ScrollbarOptions {
            content_length: 10,
            viewport_length: 10,
            ..Default::default()
        };
        assert!(axis_geometry(area(), &options).is_none());
    }

    #[test]
    fn large_content_reaches_the_full_usize_range() {
        let options = ScrollbarOptions {
            content_length: 200_000,
            viewport_length: 10,
            position: 150_000,
            ..Default::default()
        };
        let geometry = axis_geometry(area(), &options).unwrap();
        assert_eq!(geometry.offset, 150_000);
        assert_eq!(
            geometry.offset_for_thumb_center(f64::from(geometry.usable)),
            199_990
        );
    }

    #[test]
    fn empty_and_arrow_only_tracks_have_no_hit_geometry() {
        let options = ScrollbarOptions {
            content_length: 100,
            viewport_length: 10,
            ..Default::default()
        };
        for area in [
            Rect::new(0, 0, 0, 24),
            Rect::new(0, 0, 80, 0),
            Rect::new(0, 0, 80, 4),
        ] {
            assert!(axis_geometry(area, &options).is_none());
        }
    }

    #[test]
    fn arrowless_track_uses_every_cell_for_geometry() {
        let options = ScrollbarOptions {
            content_length: 100,
            viewport_length: 10,
            scrollbar: ScrollbarStyle {
                arrows: false,
                ..Default::default()
            },
            ..Default::default()
        };
        let geometry = axis_geometry(area(), &options).unwrap();
        assert_eq!(geometry.begin_length, 0);
        assert_eq!(geometry.usable, 22);
        assert_eq!(geometry.offset_for_thumb_center(22.0), 90);
        assert_eq!(
            geometry.along_clamped(ratatui::layout::Position::new(79, 1), true),
            0.0
        );
    }

    #[test]
    fn axis_geometry_maps_units_to_scrollable_range() {
        let options = ScrollbarOptions {
            content_length: 100,
            viewport_length: 10,
            position: 90,
            ..Default::default()
        };
        let geometry = axis_geometry(area(), &options).expect("scrollable");
        assert_eq!(geometry.scrollable, 90);
        assert_eq!(geometry.offset, 90);
        // 滑块应停在可用段末端(position = 最大偏移)。
        assert_eq!(
            geometry.thumb_start(),
            geometry.usable - geometry.thumb_len()
        );
    }
}

#[cfg(test)]
mod interaction_tests {
    use super::*;
    use crate::mouse_test::Mounted;
    use crate::prelude::Fragment;
    use crate::{AnyElement, UseState};
    use crossterm::event::{MouseButton, MouseEventKind};
    use ratatui_kit_macros::{Props, component, element};
    use std::sync::{Arc, Mutex};

    type Log = Arc<Mutex<Vec<usize>>>;

    #[derive(Clone, Default, Props)]
    struct HarnessProps {
        log: Log,
    }

    // 根组件 80x24,右侧纵向滚动条:滑轨 x=79、y∈[1,23),扣首尾箭头后 usable=20。
    // content=100/viewport=10 → scrollable=90(max offset 90)。
    #[component]
    fn Harness(props: &HarnessProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let position = hooks.use_state(|| 0usize);
        let log = props.log.clone();
        hooks.use_scrollbar(
            ScrollbarOptions {
                content_length: 100,
                viewport_length: 10,
                position: position.get(),
                ..Default::default()
            },
            {
                let mut position = position;
                move |target: usize| {
                    position.set(target);
                    log.lock().unwrap().push(target);
                }
            },
        );
        element!(Fragment)
    }

    #[component]
    fn RenderHarness(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        hooks.use_scrollbar(
            ScrollbarOptions {
                content_length: 100,
                viewport_length: 10,
                position: 37,
                scrollbar: ScrollbarStyle {
                    symbols: Some(ratatui::symbols::scrollbar::Set {
                        thumb: "█",
                        track: "░",
                        begin: "↑",
                        end: "↓",
                    }),
                    ..Default::default()
                },
                ..Default::default()
            },
            |_| {},
        );
        element!(Fragment)
    }

    #[test]
    fn rendered_thumb_matches_hit_geometry_with_custom_viewport() {
        let mut m = Mounted::mount_sized(element!(RenderHarness), true, 80, 24);
        let geometry = axis_geometry(
            Rect::new(0, 0, 80, 24),
            &ScrollbarOptions {
                content_length: 100,
                viewport_length: 10,
                position: 37,
                ..Default::default()
            },
        )
        .unwrap();
        let buffer = m.buffer();
        let thumb_rows: Vec<u16> = (1..23)
            .filter(|&y| buffer[(79, y)].symbol() == "█")
            .collect();
        assert_eq!(buffer[(79, 1)].symbol(), "↑");
        assert_eq!(buffer[(79, 22)].symbol(), "↓");
        let start = 2 + geometry.thumb_start();
        assert_eq!(
            thumb_rows,
            (start..start + geometry.thumb_len()).collect::<Vec<_>>()
        );
    }

    fn mount(log: &Log) -> Mounted {
        Mounted::mount_sized(crate::element!(Harness(log: log.clone())), true, 80, 24)
    }

    fn click(m: &mut Mounted, x: u16, y: u16) {
        m.dispatch(MouseEventKind::Down(MouseButton::Left), x, y);
        m.dispatch(MouseEventKind::Up(MouseButton::Left), x, y);
    }

    /// 点滑轨空白 = 滑块中心对齐指针:mid track (79,12) → usable 坐标 10。
    #[test]
    fn clicking_track_jumps_thumb_center_to_pointer() {
        let log: Log = Default::default();
        let mut m = mount(&log);
        click(&mut m, 79, 12);
        // offset = round((10 - thumb_len/2) * 100 / 20),thumb_len = 2。
        assert_eq!(log.lock().unwrap().as_slice(), [45]);
    }

    /// 抓住滑块从顶部拖到底:两端精确(0 → max offset)。
    #[test]
    fn dragging_thumb_reaches_both_ends_exactly() {
        let log: Log = Default::default();
        let mut m = mount(&log);

        m.dispatch(MouseEventKind::Down(MouseButton::Left), 79, 2); // 滑块顶部(行 1 是箭头)
        assert!(log.lock().unwrap().is_empty(), "抓住滑块不改变偏移");
        m.dispatch(MouseEventKind::Drag(MouseButton::Left), 79, 22); // 拖到底
        assert_eq!(*log.lock().unwrap().last().unwrap(), 90);
        m.dispatch(MouseEventKind::Up(MouseButton::Left), 79, 22);
    }

    /// 点在滑轨之外放行:不触发 on_seek。
    #[test]
    fn clicking_off_track_is_ignored() {
        let log: Log = Default::default();
        let mut m = mount(&log);
        click(&mut m, 10, 12);
        assert!(log.lock().unwrap().is_empty());
    }
}
