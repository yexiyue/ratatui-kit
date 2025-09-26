use ratatui::TerminalOptions;

use super::{Element, ElementKey, element_ext::ElementExt};
use crate::{
    component::{Component, ComponentHelper, ComponentHelperExt},
    props::AnyProps,
    render::tree::render_loop,
    terminal::{CrossTerminal, Terminal},
};
use std::io;

pub struct AnyElement<'a> {
    key: ElementKey,
    props: AnyProps<'a>,
    helper: Box<dyn ComponentHelperExt>,
}

impl<'a, T> From<Element<'a, T>> for AnyElement<'a>
where
    T: Component,
{
    fn from(value: Element<'a, T>) -> Self {
        Self {
            key: value.key,
            props: AnyProps::owned(value.props),
            helper: ComponentHelper::<T>::boxed(),
        }
    }
}

impl<'a, 'b: 'a, T> From<&'a mut Element<'b, T>> for AnyElement<'a>
where
    T: Component,
{
    fn from(value: &'a mut Element<'b, T>) -> Self {
        Self {
            key: value.key.clone(),
            props: AnyProps::borrowed(&mut value.props),
            helper: ComponentHelper::<T>::boxed(),
        }
    }
}

impl<'a, 'b: 'a> From<&'a mut AnyElement<'b>> for AnyElement<'b> {
    fn from(value: &'a mut AnyElement<'b>) -> Self {
        Self {
            key: value.key.clone(),
            props: value.props.borrow(),
            helper: value.helper.copy(),
        }
    }
}

impl<'a> ElementExt for AnyElement<'a> {
    fn key(&self) -> &ElementKey {
        &self.key
    }

    fn helper(&self) -> Box<dyn ComponentHelperExt> {
        self.helper.copy()
    }

    fn props_mut(&'_ mut self) -> AnyProps<'_> {
        self.props.borrow()
    }

    async fn render_loop(&mut self, options: TerminalOptions) -> io::Result<()> {
        let terminal = Terminal::new(CrossTerminal::with_options(options)?)?;
        render_loop(self, terminal).await?;
        Ok(())
    }

    async fn fullscreen(&mut self) -> io::Result<()> {
        let terminal = Terminal::new(CrossTerminal::new()?)?;
        render_loop(self, terminal).await?;
        Ok(())
    }
}

impl<'a> ElementExt for &mut AnyElement<'a> {
    fn key(&self) -> &ElementKey {
        &self.key
    }

    fn helper(&self) -> Box<dyn ComponentHelperExt> {
        self.helper.copy()
    }

    fn props_mut(&'_ mut self) -> AnyProps<'_> {
        self.props.borrow()
    }

    async fn render_loop(&mut self, options: TerminalOptions) -> io::Result<()> {
        let terminal = Terminal::new(CrossTerminal::with_options(options)?)?;
        render_loop(&mut **self, terminal).await?;
        Ok(())
    }

    async fn fullscreen(&mut self) -> io::Result<()> {
        let terminal = Terminal::new(CrossTerminal::new()?)?;
        render_loop(&mut **self, terminal).await?;
        Ok(())
    }
}
