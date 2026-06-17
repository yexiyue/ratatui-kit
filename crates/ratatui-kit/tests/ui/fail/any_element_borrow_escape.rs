use ratatui_kit::prelude::*;

fn leak_any_element<'short>(source: &'short mut AnyElement<'static>) -> AnyElement<'static> {
    AnyElement::from(source)
}

fn main() {}
