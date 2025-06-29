use super::{Element, ElementKey, element_ext::ElementExt};
use crate::{
    component::{Component, ComponentHelper, ComponentHelperExt},
    props::AnyProps,
    render::tree::{render, render_loop},
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

    fn props_mut(&mut self) -> AnyProps {
        self.props.borrow()
    }

    fn render(&mut self) -> io::Result<()> {
        let terminal = Terminal::new(CrossTerminal::new(false)?);
        render(self, terminal)?;
        Ok(())
    }

    async fn render_loop(&mut self) -> io::Result<()> {
        let terminal = Terminal::new(CrossTerminal::new(false)?);
        render_loop(self, terminal).await?;
        Ok(())
    }

    async fn fullscreen(&mut self) -> io::Result<()> {
        let terminal = Terminal::new(CrossTerminal::new(true)?);
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

    fn props_mut(&mut self) -> AnyProps {
        self.props.borrow()
    }

    fn render(&mut self) -> io::Result<()> {
        let terminal = Terminal::new(CrossTerminal::new(false)?);
        render(&mut **self, terminal)?;
        Ok(())
    }

    async fn render_loop(&mut self) -> io::Result<()> {
        let terminal = Terminal::new(CrossTerminal::new(false)?);
        render_loop(&mut **self, terminal).await?;
        Ok(())
    }

    async fn fullscreen(&mut self) -> io::Result<()> {
        let terminal = Terminal::new(CrossTerminal::new(true)?);
        render_loop(&mut **self, terminal).await?;
        Ok(())
    }
}
