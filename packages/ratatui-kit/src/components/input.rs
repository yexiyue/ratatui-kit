use crate::{
    AnyElement, Hooks, UsePreviousSize,
    prelude::{Fragment, Positioned, Text, View},
};
use ratatui::{layout::Constraint, style::Style, text::Span};
use ratatui_kit_macros::{Props, component, element};

#[derive(Debug, Clone, Props, Default)]
pub struct InputProps {
    pub input: tui_input::Input,
    pub cursor_style: Style,
    pub placeholder: String,
    pub placeholder_style: Style,
    pub style: Style,
    pub hide_cursor: bool,
}

#[component]
pub fn Input(props: &InputProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let size = hooks.use_previous_size();
    let input = &props.input;
    let scroll = input.visual_scroll(size.width.saturating_sub(1) as usize);
    let text = if input.value().is_empty() {
        props.placeholder.clone()
    } else {
        input.value().to_string()
    };

    let x = input.visual_cursor().max(scroll) - scroll;

    let position = (size.x + x as u16, size.y);

    element!(View{
        Fragment{
            #(if !props.hide_cursor{
                element!(Positioned(
                    x: position.0.min(size.x + size.width.saturating_sub(1)),
                    y: position.1.min(size.y + size.height),
                    width: 1u16,
                    height: 1u16,
                ){
                    $Span::from(" ").style(props.cursor_style)
                }).into_any()
            }else{
                element!(View(height:Constraint::Length(0),width:Constraint::Length(0))).into_any()
            })
        }
        Text(
            text:text,
            style: if input.value().is_empty() {
                props.placeholder_style
            }else{
                props.style
            },
            scroll:(0, scroll as u16),
        )
    })
}
