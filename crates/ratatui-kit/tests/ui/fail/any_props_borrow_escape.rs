use ratatui_kit::prelude::*;

fn leak_any_props<'short>(element: &'short mut Element<'static, View>) -> AnyProps<'static> {
    element.props_mut()
}

fn main() {}
