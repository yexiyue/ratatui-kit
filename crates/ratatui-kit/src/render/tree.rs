use futures::{
    FutureExt,
    future::{Either, select},
};
use std::io::{self};

use crate::{
    ElementKey,
    component::{ComponentHelperExt, InstantiatedComponent},
    context::{ContextStack, SystemContext},
    element::{ElementRepr, RunOptions},
    props::AnyProps,
    terminal::{CrossTerminal, Terminal, TerminalImpl, UpdaterTerminal},
};

use super::ComponentDrawer;

// 退出时恢复终端状态:启用过鼠标捕获则先解除捕获,再走 ratatui 的常规恢复。
// 解除失败(如 stdout 已不可用)不阻断后续 restore,仅忽略错误。
struct RestoreGuard {
    mouse: bool,
}

impl Drop for RestoreGuard {
    fn drop(&mut self) {
        if self.mouse {
            let _ = crate::terminal::set_mouse_capture(false);
        }
        ratatui::restore();
    }
}

fn should_quit_on_ctrl_c(system_context: &SystemContext, event: &crossterm::event::Event) -> bool {
    system_context.auto_quit_on_ctrl_c() && CrossTerminal::received_ctrl_c(event.clone())
}

#[doc(hidden)]
pub struct Tree<'a> {
    root_component: InstantiatedComponent,
    props: AnyProps<'a>,
    system_context: SystemContext,
}

impl<'a> Tree<'a> {
    pub(crate) fn new_with_mouse(
        mut props: AnyProps<'a>,
        helper: Box<dyn ComponentHelperExt>,
        mouse: bool,
    ) -> Self {
        Tree {
            root_component: InstantiatedComponent::new(
                ElementKey::user("_root_tree_"),
                props.borrow(),
                helper,
            ),
            props,
            system_context: SystemContext::new_with_mouse(mouse),
        }
    }

    // 只跑一次 update（自顶向下运行组件、协调子树）。终端以对象安全的
    // `&mut dyn UpdaterTerminal` 传入,故渲染 harness 可用 no-op 终端驱动。
    pub(crate) fn update_once(&mut self, terminal: &mut dyn UpdaterTerminal) {
        // 每帧重建输入注册表（清空上一帧层/handler、铸造 root 层)。
        // 必须在 ContextStack::root 借走 &mut system_context 之前完成,二者借用不重叠。
        self.system_context.input.begin_frame();
        let mut component_context_stack = ContextStack::root(&mut self.system_context);
        self.root_component
            .update(terminal, &mut component_context_stack, self.props.borrow());
    }

    // 只跑一次 draw（把树绘到给定 drawer）。供渲染 harness 直接画到 TestBackend Buffer。
    pub(crate) fn draw_root(&mut self, drawer: &mut ComponentDrawer) {
        self.root_component.draw(drawer);
    }

    // 测试专用:驱动事件分发(mouse_test harness)。
    #[cfg(test)]
    pub(crate) fn system_context_mut(&mut self) -> &mut SystemContext {
        &mut self.system_context
    }

    fn render<T: TerminalImpl<Event = crossterm::event::Event>>(
        &mut self,
        terminal: &mut Terminal<T>,
    ) -> io::Result<()> {
        self.update_once(terminal);

        terminal.draw(|frame| {
            let area = frame.area();
            let mut drawer = ComponentDrawer::new(frame, area);
            self.draw_root(&mut drawer);
        })?;

        Ok(())
    }

    async fn render_loop<T: TerminalImpl<Event = crossterm::event::Event>>(
        &mut self,
        terminal: &mut Terminal<T>,
    ) -> io::Result<()> {
        let mut skip_render = false;
        loop {
            if !skip_render {
                self.render(terminal)?;
            }
            // 首帧 update 完成后再检查注册,确保无需额外键盘/鼠标事件就能开始 hover。
            // 惰性升级 any-motion 上报:存在 hover/Move 追踪注册时补开 1003h(只升不降)。
            // 门控 mouse_enabled:未启用捕获时不得改动终端模式(行为与现状完全一致)。
            if self.system_context.mouse_enabled()
                && self.system_context.input.needs_any_motion_upgrade()
            {
                terminal.enable_any_motion()?;
                self.system_context.input.mark_any_motion_enabled();
            }
            skip_render = false;
            if self.system_context.should_exit() {
                break;
            }
            match select(
                self.root_component.wait().boxed_local(),
                terminal.next_event().boxed_local(),
            )
            .await
            {
                // 组件树/状态变更：仅回到循环顶重渲染。
                Either::Left(((), _)) => continue,
                // 取到一个 raw 事件。
                Either::Right((Some(event), _)) => {
                    if should_quit_on_ctrl_c(&self.system_context, &event) {
                        break;
                    }
                    let repaint = self.system_context.input.dispatch(event);
                    if repaint || self.system_context.should_exit() {
                        continue;
                    }
                    // Moved 无消费且 hover 无变化:跳过本轮重绘直接回 select。
                    // handler 若写 State,`root_component.wait()` 的 waker 会立即拉起重绘,
                    // 不依赖每事件无条件重绘(observer 型 handler 不得产生副作用的契约由此强化)。
                    skip_render = true;
                    continue;
                }
                // 事件流结束。
                Either::Right((None, _)) => break,
            }
        }
        Ok(())
    }
}

pub(crate) async fn render_loop<E: ElementRepr>(
    mut element: E,
    mut terminal: Terminal,
    run: RunOptions,
) -> io::Result<()> {
    let helper = element.helper();
    let mut tree = Tree::new_with_mouse(element.props_mut(), helper, run.mouse);
    let _restore_guard = RestoreGuard { mouse: run.mouse };

    tree.render_loop(&mut terminal).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

    fn ctrl_c_event() -> Event {
        Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL))
    }

    #[test]
    fn ctrl_c_quits_by_default() {
        assert!(should_quit_on_ctrl_c(
            &SystemContext::new_with_mouse(false),
            &ctrl_c_event()
        ));
    }

    #[test]
    fn ctrl_c_is_dispatched_when_auto_quit_is_disabled() {
        let mut system_context = SystemContext::new_with_mouse(false);
        system_context.set_auto_quit_on_ctrl_c(false);

        assert!(!should_quit_on_ctrl_c(&system_context, &ctrl_c_event()));
    }

    #[test]
    fn regular_key_does_not_quit() {
        let event = Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE));

        assert!(!should_quit_on_ctrl_c(
            &SystemContext::new_with_mouse(false),
            &event
        ));
    }
    // ---- 渲染循环级测试:脚本化终端按序吐事件,统计 draw 次数 ----

    use crate::{
        AnyElement, Hooks, UseEventHandler,
        components::Text,
        input::{EventOptions, EventPriority, EventResult, EventScope},
        terminal::TerminalImpl,
    };
    use crossterm::event::{MouseEvent, MouseEventKind};
    use futures::{StreamExt, stream::BoxStream};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    struct ScriptedTerminal {
        events: Vec<Event>,
        draws: Arc<AtomicUsize>,
        motion_upgrades: Arc<AtomicUsize>,
    }

    impl TerminalImpl for ScriptedTerminal {
        type Event = Event;

        fn event_stream(&mut self) -> io::Result<BoxStream<'static, Self::Event>> {
            Ok(futures::stream::iter(std::mem::take(&mut self.events)).boxed())
        }

        fn received_ctrl_c(event: Self::Event) -> bool {
            CrossTerminal::received_ctrl_c(event)
        }

        fn enable_any_motion(&mut self) -> io::Result<()> {
            self.motion_upgrades.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }

        fn draw<F>(&mut self, _f: F) -> io::Result<()>
        where
            F: FnOnce(&mut ratatui::Frame),
        {
            self.draws.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }

        fn insert_before<F>(&mut self, _height: u16, _draw_fn: F) -> io::Result<()>
        where
            F: FnOnce(&mut ratatui::buffer::Buffer),
        {
            Ok(())
        }
    }

    fn moved(col: u16, row: u16) -> Event {
        Event::Mouse(MouseEvent {
            kind: MouseEventKind::Moved,
            column: col,
            row,
            modifiers: KeyModifiers::NONE,
        })
    }

    // 消费 Moved 的组件:每个 Moved 都应触发一次重绘。
    #[crate::component]
    fn ConsumeMoved(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        hooks.use_event_handler_with_options(
            EventScope::Current,
            EventPriority::Normal,
            EventOptions { hit_test: false },
            |event| {
                if matches!(
                    event,
                    Event::Mouse(crossterm::event::MouseEvent {
                        kind: MouseEventKind::Moved,
                        ..
                    })
                ) {
                    EventResult::Consumed
                } else {
                    EventResult::Ignored
                }
            },
        );
        crate::element!(Text(text: "x"))
    }

    #[tokio::test]
    async fn moved_without_consumer_skips_re_render() {
        let draws = Arc::new(AtomicUsize::new(0));
        let mut terminal = Terminal::new(ScriptedTerminal {
            events: vec![moved(1, 1), moved(2, 2), ctrl_c_event()],
            draws: draws.clone(),
            motion_upgrades: Arc::default(),
        })
        .unwrap();

        let mut el = crate::element!(Text(text: "x"));
        let helper = el.helper();
        let mut tree = Tree::new_with_mouse(el.props_mut(), helper, false);
        tree.render_loop(&mut terminal).await.unwrap();

        // 无人消费、无 hover 变化的 Moved 不触发重绘:只有初始帧画了一次。
        assert_eq!(draws.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn moved_with_consumer_re_renders() {
        let draws = Arc::new(AtomicUsize::new(0));
        let mut terminal = Terminal::new(ScriptedTerminal {
            events: vec![moved(1, 1), moved(2, 2), ctrl_c_event()],
            draws: draws.clone(),
            motion_upgrades: Arc::default(),
        })
        .unwrap();

        let mut el = crate::element!(ConsumeMoved);
        let helper = el.helper();
        let mut tree = Tree::new_with_mouse(el.props_mut(), helper, false);
        tree.render_loop(&mut terminal).await.unwrap();

        // 初始帧 + 两个被消费的 Moved 各一次重绘。
        assert_eq!(draws.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn run_options_default_disables_mouse() {
        assert!(!crate::RunOptions::default().mouse);
    }

    #[crate::component]
    fn HoverOnMount(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        use crate::UseHover;
        let _ = hooks.use_hover();
        crate::element!(Text(text: "hover"))
    }

    #[tokio::test]
    async fn hover_on_mount_enables_motion_before_first_event() {
        let motion_upgrades = Arc::new(AtomicUsize::new(0));
        let mut terminal = Terminal::new(ScriptedTerminal {
            events: vec![ctrl_c_event()],
            draws: Arc::default(),
            motion_upgrades: motion_upgrades.clone(),
        })
        .unwrap();
        let mut el = crate::element!(HoverOnMount);
        let helper = el.helper();
        let mut tree = Tree::new_with_mouse(el.props_mut(), helper, true);
        tree.render_loop(&mut terminal).await.unwrap();
        assert_eq!(motion_upgrades.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn tree_with_mouse_configures_system_context() {
        let mut el = crate::element!(Text(text: "x"));
        let helper = el.helper();
        let tree = Tree::new_with_mouse(el.props_mut(), helper, true);
        assert!(tree.system_context.mouse_enabled());
    }
}
