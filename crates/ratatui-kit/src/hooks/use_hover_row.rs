// use_hover_row hook:自定义绘制内容的「悬停行/条目」跟踪。
//
// 适用场景:直接驱动第三方列表 widget(如 tui_widget_list)或不经内置组件的
// 自绘内容 —— 调用方提供「指针位置 → 条目标识」的 hit 闭包,hook 负责注册
// 指针流(Enter/Move/Leave,track_move)并维护悬停值,渲染闭包读返回的
// State 即可自绘悬停态。与内置组件(Select/VirtualList 等)的悬停语义一致:
// 仅变化时写 State,移动高频但不产生无谓重绘。

use crate::{
    EventResult, Hooks, PointerKind, PointerOptions, State, UseState,
    hooks::use_pointer::{pointer_enabled, use_pointer_with_layout_sync},
};
use ratatui::layout::Position;

mod private {
    pub trait Sealed {}
    impl Sealed for crate::hooks::Hooks<'_, '_> {}
}

pub trait UseHoverRow: private::Sealed {
    /// 跟踪指针悬停的行/条目:`hit` 把指针位置映射为条目标识(行号、节点 id 等;
    /// 返回 `None` 表示该位置无条目,如条目间隙/视口尾部空白)。返回的 State 在
    /// Enter/Move 时更新为命中值,指针移出(`hit` 恒 `None`)或 Leave 时清为
    /// `None`;**仅变化时写入**(移动事件高频,避免每格重绘)。
    ///
    /// hook 注册在调用组件上,指针事件按该组件区域命中过滤。仅应用启用鼠标捕获
    /// 时注册指针流;`hit` 闭包内可自行做 active/条目数等门控(返回 None 即不悬停)。
    ///
    /// **手写 `Component::update` 需先接 context stack**
    /// (`hooks.with_context_stack(updater.component_context_stack())`):否则
    /// `pointer_enabled` 读不到 `SystemContext`,本 hook 静默不注册(不悬停、无报错;
    /// debug 构建会断言)。`#[component]` fn 组件由宏自动接,无此要求。
    fn use_hover_row<T, F>(&mut self, hit: F) -> State<Option<T>>
    where
        T: PartialEq + Send + Sync + Unpin + 'static,
        F: Fn(Position) -> Option<T> + 'static;
}

impl UseHoverRow for Hooks<'_, '_> {
    fn use_hover_row<T, F>(&mut self, hit: F) -> State<Option<T>>
    where
        T: PartialEq + Send + Sync + Unpin + 'static,
        F: Fn(Position) -> Option<T> + 'static,
    {
        let hover = self.use_state(|| None::<T>);
        // 条件注册(mouse_enabled 进程内不变,分支跨帧稳定,不违反 hook 顺序)。
        if !pointer_enabled(self) {
            return hover;
        }
        use_pointer_with_layout_sync(
            self,
            PointerOptions {
                // 悬停跟踪需要 Move/Enter/Leave 上报。
                track_move: true,
                ..Default::default()
            },
            {
                let mut hover = hover;
                move |ev| match ev.kind {
                    PointerKind::Enter | PointerKind::Move => {
                        let next = hit(ev.position);
                        // 读 guard 在分号处释放后再写,避免读写借用交叠。
                        let changed = hover.read().ne(&next);
                        if changed {
                            hover.set(next);
                            EventResult::Consumed
                        } else {
                            EventResult::Ignored
                        }
                    }
                    PointerKind::Leave => {
                        let empty = hover.read().is_none();
                        if !empty {
                            hover.set(None);
                        }
                        EventResult::Ignored
                    }
                    _ => EventResult::Ignored,
                }
            },
        );
        hover
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mouse_test::Mounted;
    use crate::prelude::{Fragment, Text};
    use crate::{AnyElement, UsePointer, UseState};
    use crossterm::event::MouseEventKind;
    use ratatui_kit_macros::{Props, component, element};
    use std::cell::RefCell;
    use std::rc::Rc;

    // 40x10 终端,行 0..4 各算一个条目,其余位置无条目。
    fn hit(pos: Position) -> Option<usize> {
        (pos.y < 5).then_some(pos.y as usize)
    }

    #[derive(Clone, Default, Props)]
    struct HarnessProps {
        label: &'static str,
    }

    #[component]
    fn Harness(props: &HarnessProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let hover = hooks.use_hover_row(hit);
        let text = format!("{}:{:?}", props.label, *hover.read());
        element!(Fragment {
            Text(text: crate::components::TextParagraph::from(text))
        })
    }

    #[test]
    fn tracks_hovered_row_and_clears_when_moving_off() {
        let mut m = Mounted::mount(crate::element!(Harness(label: "h")), true);

        m.dispatch(MouseEventKind::Moved, 5, 2);
        assert!(
            crate::mouse_test::find_text(&m.buffer(), "h:Some(2)").is_some(),
            "悬停行 2 应映射为 Some(2)"
        );

        // 移到无条目区域(hit = None)→ 清空。
        m.dispatch(MouseEventKind::Moved, 5, 7);
        assert!(
            crate::mouse_test::find_text(&m.buffer(), "h:None").is_some(),
            "移出条目区应清为 None"
        );
    }

    #[test]
    fn moving_within_the_same_row_does_not_request_redraw() {
        let mut m = Mounted::mount(crate::element!(Harness(label: "h")), true);
        assert!(m.dispatch(MouseEventKind::Moved, 5, 2));
        assert!(!m.dispatch(MouseEventKind::Moved, 6, 2));
        assert!(m.dispatch(MouseEventKind::Moved, 6, 3));
    }

    #[component]
    fn ChangingRows(_props: &HarnessProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        use crate::{EventResult, UseEventHandler};
        let mut base = hooks.use_state(|| 0usize);
        hooks.use_event_handler(crate::input::EventScope::Current, crate::input::EventPriority::Normal, move |event| {
            if matches!(event, crossterm::event::Event::Key(key) if key.code == crossterm::event::KeyCode::Char('n')) {
                base.set(10);
                EventResult::Consumed
            } else {
                EventResult::Ignored
            }
        });
        let hover = hooks.use_hover_row(move |position| Some(base.get() + position.y as usize));
        element!(Text(text: format!("row:{:?}", *hover.read())))
    }

    #[test]
    fn stationary_pointer_tracks_changed_row_mapping() {
        let mut mounted = Mounted::mount(element!(ChangingRows), true);
        mounted.dispatch(MouseEventKind::Moved, 3, 2);
        assert!(crate::mouse_test::find_text(&mounted.buffer(), "row:Some(2)").is_some());
        mounted.dispatch_key(crossterm::event::KeyCode::Char('n'));
        assert!(crate::mouse_test::find_text(&mounted.buffer(), "row:Some(12)").is_some());
    }

    #[derive(Default, Props)]
    struct PairProps;

    #[component]
    fn Pair(_props: &PairProps, hooks: Hooks) -> impl Into<AnyElement<'static>> {
        element!(crate::components::View(flex_direction: ratatui::layout::Direction::Horizontal) {
            Harness(label: "left")
            Harness(label: "right")
        })
    }

    #[test]
    fn moving_between_siblings_clears_previous_hover_in_the_same_event() {
        let mut mounted = Mounted::mount(element!(Pair), true);
        mounted.dispatch(MouseEventKind::Moved, 1, 2);
        assert!(crate::mouse_test::find_text(&mounted.buffer(), "left:Some(2)").is_some());
        mounted.dispatch(MouseEventKind::Moved, 21, 2);
        assert!(crate::mouse_test::find_text(&mounted.buffer(), "left:None").is_some());
        assert!(crate::mouse_test::find_text(&mounted.buffer(), "right:Some(2)").is_some());
        mounted.dispatch(MouseEventKind::Moved, 1, 3);
        assert!(crate::mouse_test::find_text(&mounted.buffer(), "right:None").is_some());
    }

    // ---- TRNovel ListSelect→ListView 场景回归 ----
    //
    // 复刻自建列表组件的常见结构:fn 组件(自带 use_pointer 处理点击/滚轮,Move 放行)
    // 包裹手写 `Component`(use_hover_row + 渲染期记录条目位置,事件读取上一帧记录)。
    // 任何一层(注册层、分发序、状态更新)断裂都会让悬停失效。

    /// 条目位置记录:draw 时写入,事件回调读取(模拟 tui-widget-list 的渲染期记录)。
    #[derive(Clone, Default)]
    struct ItemAreas(Rc<RefCell<Vec<ratatui::layout::Rect>>>);

    #[derive(Default, Props)]
    struct ReproListProps {
        areas: ItemAreas,
    }

    struct ReproList {
        hover: Option<State<Option<usize>>>,
        areas: ItemAreas,
    }

    impl crate::Component for ReproList {
        type Props<'a>
            = ReproListProps
        where
            Self: 'a;

        fn new(props: &Self::Props<'_>) -> Self {
            Self {
                hover: None,
                areas: props.areas.clone(),
            }
        }

        fn update(
            &mut self,
            props: &mut Self::Props<'_>,
            mut hooks: Hooks,
            updater: &mut crate::ComponentUpdater,
        ) {
            // 手写 Component 的必需步骤:不接 context stack 时 pointer_enabled 读不到
            // SystemContext,use_hover_row 的指针注册会被静默跳过(本测试即此场景回归)。
            let mut hooks = hooks.with_context_stack(updater.component_context_stack());
            let areas = props.areas.clone();
            let hover = hooks
                .use_hover_row(move |pos| areas.0.borrow().iter().position(|r| r.contains(pos)));
            *self = Self {
                hover: Some(hover),
                areas: props.areas.clone(),
            };
        }

        fn draw(&mut self, drawer: &mut crate::ComponentDrawer<'_, '_>) {
            // 3 个条目,每个 3 行高(0..3 / 3..6 / 6..9);悬停行的标签加 H 前缀。
            let mut areas = Vec::new();
            for i in 0..3u16 {
                let rect = ratatui::layout::Rect::new(
                    drawer.area.x,
                    drawer.area.y + i * 3,
                    drawer.area.width,
                    3,
                );
                areas.push(rect);
                let hovered = self.hover.as_ref().and_then(|h| h.get()) == Some(i as usize);
                let mark = if hovered { "H" } else { "-" };
                let text = ratatui::widgets::Paragraph::new(format!("{mark}item{i}"));
                drawer.render_widget(text, rect);
            }
            *self.areas.0.borrow_mut() = areas;
        }
    }

    #[derive(Default, Props)]
    struct WrapProps {
        areas: ItemAreas,
    }

    // 模拟 ListSelect:自身注册 use_pointer(命中过滤;点击/滚轮消费,Move 放行)。
    #[component]
    fn WrapList(props: &WrapProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        use crate::input::EventResult as _;
        hooks.use_pointer(PointerOptions::default(), |ev| match ev.kind {
            PointerKind::Click { .. } | PointerKind::ScrollUp | PointerKind::ScrollDown => {
                EventResult::Consumed
            }
            _ => EventResult::Ignored,
        });
        element!(ReproList(areas: props.areas.clone()))
    }

    #[test]
    fn hover_row_survives_pointer_wrapper_component() {
        let areas = ItemAreas::default();
        let mut m = Mounted::mount(element!(WrapList(areas: areas.clone())), true);

        m.dispatch(MouseEventKind::Moved, 5, 1);
        assert!(
            crate::mouse_test::find_text(&m.buffer(), "Hitem0").is_some(),
            "悬停首条目应显示 H 标记"
        );

        m.dispatch(MouseEventKind::Moved, 5, 4);
        assert!(
            crate::mouse_test::find_text(&m.buffer(), "Hitem1").is_some(),
            "移到第二条目"
        );

        // 移出全部条目(视口尾部空白)→ 清空。
        m.dispatch(MouseEventKind::Moved, 5, 9);
        assert!(
            crate::mouse_test::find_text(&m.buffer(), "-item0").is_some()
                && crate::mouse_test::find_text(&m.buffer(), "Hitem").is_none(),
            "移出条目区应清为无悬停"
        );
    }
}
