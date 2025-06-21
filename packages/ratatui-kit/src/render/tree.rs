use futures::{FutureExt, future::select};
use std::io::{self};

use crate::{
    ElementKey,
    component::{ComponentHelperExt, InstantiatedComponent},
    context::{ContextStack, SystemContext},
    element::ElementExt,
    props::AnyProps,
    terminal::Terminal,
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
                ElementKey::new("_root_tree_"),
                props.borrow(),
                helper,
            ),
            props,
            system_context: SystemContext::new(),
        }
    }

    fn render(&mut self, terminal: &mut Terminal) -> io::Result<()> {
        let mut component_context_stack = ContextStack::root(&mut self.system_context);
        self.root_component
            .update(terminal, &mut component_context_stack, self.props.borrow());

        terminal
            .draw(|frame| {
                let area = frame.area();
                let mut drawer = ComponentDrawer::new(frame, area);
                self.root_component.draw(&mut drawer);
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
            select(self.root_component.wait().boxed(), terminal.wait().boxed()).await;
            if terminal.received_ctrl_c() {
                break;
            }
        }
        Ok(())
    }
}

pub(crate) fn render<E: ElementExt>(mut element: E, mut terminal: Terminal) -> io::Result<()> {
    let helper = element.helper();
    let mut tree = Tree::new(element.props_mut(), helper);

    terminal.events()?;

    tree.render(&mut terminal)?;
    Ok(())
}

pub(crate) async fn render_loop<E: ElementExt>(
    mut element: E,
    mut terminal: Terminal,
) -> io::Result<()> {
    let helper = element.helper();
    let mut tree = Tree::new(element.props_mut(), helper);

    terminal.events()?;

    tree.render_loop(&mut terminal).await?;
    Ok(())
}
