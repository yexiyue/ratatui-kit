use crate::{Component, Handler, Hooks, UseEvents};
use ratatui::style::Style;
use ratatui_kit_macros::Props;
use std::{
    borrow::Cow,
    sync::{Arc, RwLock},
};
use tui_textarea::{Input, Key, TextArea as TUITextArea};

#[derive(Props, Default)]
pub struct TextAreaProps<'a> {
    pub value: Cow<'a, str>,
    pub is_focus: bool,
    pub on_change: Handler<'static, String>,
    pub multiline: bool,
    pub cursor_style: Style,
    pub cursor_line_style: Style,
    pub placeholder: Option<String>,
    pub placeholder_style: Style,
    pub style: Style,
}

pub struct TextArea {
    inner: Arc<RwLock<TUITextArea<'static>>>,
}

impl Component for TextArea {
    type Props<'a> = TextAreaProps<'a>;
    fn new(props: &Self::Props<'_>) -> Self {
        let inner = TUITextArea::from(props.value.lines());

        Self {
            inner: Arc::new(RwLock::new(inner)),
        }
    }

    fn update(
        &mut self,
        props: &mut Self::Props<'_>,
        mut hooks: Hooks,
        _updater: &mut crate::ComponentUpdater,
    ) {
        hooks.use_local_events({
            let inner = self.inner.clone();
            let is_focus = props.is_focus;
            let multiline = props.multiline;
            let mut handler = props.on_change.take();
            move |event| {
                if is_focus {
                    let input = Input::from(event);
                    if !multiline {
                        if input.key == Key::Enter {
                            return;
                        }
                    }
                    inner.write().unwrap().input(input);
                    handler(inner.read().unwrap().lines().join("\n"));
                }
            }
        });

        let mut inner = self.inner.write().unwrap();
        inner.set_cursor_style(props.cursor_style);
        inner.set_cursor_line_style(props.cursor_line_style);
        inner.set_style(props.style);
        if let Some(placeholder) = &props.placeholder {
            inner.set_placeholder_text(placeholder);
            inner.set_placeholder_style(props.placeholder_style);
        }
    }

    fn draw(&mut self, drawer: &mut crate::ComponentDrawer<'_, '_>) {
        let inner = self.inner.read().unwrap();
        drawer.frame.render_widget(&*inner, drawer.area);
    }
}
