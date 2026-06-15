use futures::{FutureExt, future::select};
use std::io::{self};

use crate::{
    ElementKey,
    component::{ComponentHelperExt, InstantiatedComponent},
    context::{ContextStack, SystemContext},
    element::ElementExt,
    props::AnyProps,
    terminal::{Terminal, UpdaterTerminal},
};

use super::ComponentDrawer;

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

        terminal
            .draw(|frame| {
                let area = frame.area();
                let mut drawer = ComponentDrawer::new(frame, area);
                self.draw_root(&mut drawer);
            })
            .expect("Failed to draw the terminal");

        Ok(())
    }

    async fn render_loop(&mut self, terminal: &mut Terminal) -> io::Result<()> {
        loop {
            self.render(terminal)?;
            if self.system_context.should_exit() || terminal.received_ctrl_c() {
                break;
            }
            select(
                self.root_component.wait().boxed_local(),
                terminal.wait().boxed_local(),
            )
            .await;
            if terminal.received_ctrl_c() {
                break;
            }
        }
        Ok(())
    }
}

pub(crate) async fn render_loop<E: ElementExt>(
    mut element: E,
    mut terminal: Terminal,
) -> io::Result<()> {
    let helper = element.helper();
    let mut tree = Tree::new(element.props_mut(), helper);

    terminal.events()?;

    tree.render_loop(&mut terminal).await?;

    ratatui::restore();
    Ok(())
}
