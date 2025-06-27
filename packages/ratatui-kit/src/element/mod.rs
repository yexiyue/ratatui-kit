use crate::{
    AnyProps, Component, ComponentHelper, ComponentHelperExt, CrossTerminal, Terminal,
    tree::{render, render_loop},
};
use std::io;
mod key;
pub use key::ElementKey;
mod any_element;
pub use any_element::AnyElement;
mod element_ext;
pub use element_ext::ElementExt;
mod extend_with_elements;
pub use extend_with_elements::{ExtendWithElements, extend_with_elements};

pub trait ElementType {
    type Props<'a>
    where
        Self: 'a;
}

#[derive(Clone)]
pub struct Element<'a, T: ElementType + 'a> {
    pub key: ElementKey,
    pub props: T::Props<'a>,
}

impl<'a, T> Element<'a, T>
where
    T: Component + 'a,
{
    pub fn into_any(self) -> AnyElement<'a> {
        self.into()
    }
}

impl<'a, T> ElementExt for Element<'a, T>
where
    T: Component,
{
    fn key(&self) -> &ElementKey {
        &self.key
    }

    fn helper(&self) -> Box<dyn ComponentHelperExt> {
        ComponentHelper::<T>::boxed()
    }

    fn props_mut(&mut self) -> AnyProps {
        AnyProps::borrowed(&mut self.props)
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

impl<'a, T> ElementExt for &mut Element<'a, T>
where
    T: Component,
{
    fn key(&self) -> &ElementKey {
        &self.key
    }

    fn helper(&self) -> Box<dyn ComponentHelperExt> {
        ComponentHelper::<T>::boxed()
    }

    fn props_mut(&mut self) -> AnyProps {
        AnyProps::borrowed(&mut self.props)
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
