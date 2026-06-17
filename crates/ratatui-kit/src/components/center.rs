use crate::{AnyElement, Component, ComponentUpdater, Hooks, Props, element, prelude::View};
use ratatui::layout::{Constraint, Direction, Flex};

#[derive(Default, Props)]
pub struct CenterProps<'a> {
    pub width: Constraint,
    pub height: Constraint,
    pub children: Vec<AnyElement<'a>>,
}

/// 居中布局组件
pub struct Center;

impl Component for Center {
    type Props<'a> = CenterProps<'a>;

    fn new(_props: &Self::Props<'_>) -> Self {
        Self
    }

    fn update(
        &mut self,
        props: &mut Self::Props<'_>,
        _hooks: Hooks,
        updater: &mut ComponentUpdater,
    ) {
        updater.set_transparent_layout(true);
        updater.update_children(
            [element!(
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
                            { props.children.iter_mut() }
                        }
                    }
                }
            )],
            None,
        );
    }
}
