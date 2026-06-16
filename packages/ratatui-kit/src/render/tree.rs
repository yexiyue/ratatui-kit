use futures::{
    FutureExt,
    future::{Either, select},
};
use std::io::{self};

use crate::{
    ElementKey,
    component::{ComponentHelperExt, InstantiatedComponent},
    context::{ContextStack, SystemContext},
    element::ElementRepr,
    props::AnyProps,
    terminal::{CrossTerminal, Terminal, TerminalImpl, UpdaterTerminal},
};

use super::ComponentDrawer;

struct RestoreGuard;

impl Drop for RestoreGuard {
    fn drop(&mut self) {
        ratatui::restore();
    }
}

pub struct Tree<'a> {
    root_component: InstantiatedComponent,
    props: AnyProps<'a>,
    system_context: SystemContext,
}

impl<'a> Tree<'a> {
    pub(crate) fn new(mut props: AnyProps<'a>, helper: Box<dyn ComponentHelperExt>) -> Self {
        Tree {
            root_component: InstantiatedComponent::new(
                ElementKey::user("_root_tree_"),
                props.borrow(),
                helper,
            ),
            props,
            system_context: SystemContext::new(),
        }
    }

    /// 只跑一次 update（自顶向下运行组件、协调子树）。终端以对象安全的
    /// `&mut dyn UpdaterTerminal` 传入,故渲染 harness 可用 no-op 终端驱动。
    pub(crate) fn update_once(&mut self, terminal: &mut dyn UpdaterTerminal) {
        // 每帧重建输入注册表（清空上一帧层/handler、铸造 root 层)。
        // 必须在 ContextStack::root 借走 &mut system_context 之前完成,二者借用不重叠。
        self.system_context.input.begin_frame();
        let mut component_context_stack = ContextStack::root(&mut self.system_context);
        self.root_component
            .update(terminal, &mut component_context_stack, self.props.borrow());
    }

    /// 只跑一次 draw（把树绘到给定 drawer）。供渲染 harness 直接画到 TestBackend Buffer。
    pub(crate) fn draw_root(&mut self, drawer: &mut ComponentDrawer) {
        self.root_component.draw(drawer);
    }

    fn render(&mut self, terminal: &mut Terminal) -> io::Result<()> {
        self.update_once(terminal);

        terminal.draw(|frame| {
            let area = frame.area();
            let mut drawer = ComponentDrawer::new(frame, area);
            self.draw_root(&mut drawer);
        })?;

        Ok(())
    }

    async fn render_loop(&mut self, terminal: &mut Terminal) -> io::Result<()> {
        loop {
            self.render(terminal)?;
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
                    // ctrl_c 先于 dispatch 判定,任何层的 Consumed 都吞不掉它。
                    if CrossTerminal::received_ctrl_c(event.clone()) {
                        break;
                    }
                    self.system_context.input.dispatch(event);
                    // dispatch 后无条件 continue:纯副作用/退出型 handler 不写 State 不唤醒,
                    // 仍需回到循环顶 render + 复查 should_exit,否则会在 select 永久阻塞。
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
) -> io::Result<()> {
    let helper = element.helper();
    let mut tree = Tree::new(element.props_mut(), helper);
    let _restore_guard = RestoreGuard;

    tree.render_loop(&mut terminal).await
}
