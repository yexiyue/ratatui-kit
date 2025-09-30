use crate::{AnyElement, Props, component, element, prelude::View};
use ratatui::layout::{Constraint, Direction, Flex};

#[derive(Default, Props)]
pub struct CenterProps<'a> {
    pub width: Constraint,
    pub height: Constraint,
    pub children: Vec<AnyElement<'a>>,
}

/// 居中布局组件
#[component]
pub fn Center<'a>(props: &mut CenterProps<'a>) -> impl Into<AnyElement<'a>> {
    element!(
        View(
            justify_content:Flex::Center
        ){
            View(
               height:props.height,
               justify_content:Flex::Center,
               flex_direction:Direction::Horizontal,
            ){
                View(
                    width:props.width,
                    justify_content:Flex::Center,
                    flex_direction:Direction::Vertical,
                ){
                    #(props.children.iter_mut())
                }
            }
        }
    )
}
