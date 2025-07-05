use crate::{Component, Handler, Hooks, UseEvents};
use ratatui::{style::Style, widgets::Widget};
use ratatui_kit_macros::Props;
use std::{
    borrow::Cow,
    sync::{Arc, RwLock},
};
pub use tui_textarea::Key;
use tui_textarea::{CursorMove, Input, TextArea as TUITextArea};
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
    pub disable_keys: Vec<Key>,
    pub line_number_style: Option<Style>,
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
            let disable_keys = props.disable_keys.clone();
            let mut handler = props.on_change.take();
            move |event| {
                if is_focus {
                    let input = Input::from(event);
                    let key = input.key;

                    if !multiline && input.key == Key::Enter {
                        return;
                    }

                    if disable_keys.contains(&key) {
                        return;
                    }

                    let mut inner = inner.write().unwrap();

                    inner.input(input);

                    let mut string = inner.lines().join("\n");

                    if multiline && key == Key::Enter {
                        string.push('\n');
                    }

                    handler(string);
                }
            }
        });

        let mut inner = self.inner.write().unwrap();

        let cursor = inner.cursor();

        *inner = TUITextArea::from(props.value.lines());

        inner.move_cursor(CursorMove::Jump(cursor.0 as u16, cursor.1 as u16));
        inner.set_cursor_style(props.cursor_style);
        inner.set_cursor_line_style(props.cursor_line_style);
        inner.set_style(props.style);

        if let Some(line_number_style) = &props.line_number_style {
            inner.set_line_number_style(line_number_style.clone());
        }

        if let Some(placeholder) = &props.placeholder {
            inner.set_placeholder_text(placeholder);
            inner.set_placeholder_style(props.placeholder_style);
        }
    }

    fn draw(&mut self, drawer: &mut crate::ComponentDrawer<'_, '_>) {
        let inner = self.inner.read().unwrap();
        inner.render(drawer.area, drawer.buffer_mut());
    }
}
