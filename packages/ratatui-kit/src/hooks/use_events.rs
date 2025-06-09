use std::{pin::pin, task::Poll};

use crossterm::event::Event;
use futures::Stream;
use ratatui::layout::Rect;

use crate::{Hook, Hooks, TerminalEvents};

mod private {
    pub trait Sealed {}
    impl Sealed for crate::hooks::Hooks<'_, '_> {}
}

pub trait UseEvents: private::Sealed {
    fn use_events<F>(&mut self, f: F)
    where
        F: FnMut(Event) + Send + 'static;

    fn use_local_events<F>(&mut self, f: F)
    where
        F: FnMut(Event) + Send + 'static;
}

impl UseEvents for Hooks<'_, '_> {
    fn use_events<F>(&mut self, f: F)
    where
        F: FnMut(Event) + Send + 'static,
    {
        let h = self.use_hook(move || UseEventsImpl {
            events: None,
            component_area: Default::default(),
            in_component: false,
            f: None,
        });
        h.f = Some(Box::new(f));
    }

    fn use_local_events<F>(&mut self, f: F)
    where
        F: FnMut(Event) + Send + 'static,
    {
        let h = self.use_hook(move || UseEventsImpl {
            events: None,
            component_area: Default::default(),
            in_component: true,
            f: None,
        });
        h.f = Some(Box::new(f));
    }
}

struct UseEventsImpl {
    f: Option<Box<dyn FnMut(Event) + Send>>,
    events: Option<TerminalEvents<Event>>,
    in_component: bool,
    component_area: Rect,
}

impl Hook for UseEventsImpl {
    fn poll_change(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context,
    ) -> std::task::Poll<()> {
        while let Some(Poll::Ready(Some(event))) = self
            .events
            .as_mut()
            .map(|events| pin!(events).poll_next(cx))
        {
            let area = self.component_area;
            let in_component = self.in_component;
            if let Some(f) = &mut self.f {
                if in_component {
                    match event {
                        Event::Mouse(mouse_event) => {
                            if mouse_event.row >= area.y && mouse_event.column >= area.x {
                                let row = mouse_event.row - area.y;
                                let column = mouse_event.column - area.x;

                                if row < area.height && column < area.width {
                                    f(event);
                                }
                            }
                        }
                        _ => {
                            f(event);
                        }
                    }
                } else {
                    f(event);
                }
            }
        }
        Poll::Pending
    }

    fn post_component_update(&mut self, updater: &mut crate::ComponentUpdater) {
        if self.events.is_none() {
            self.events = updater.terminal().events().ok();
        }
    }

    fn post_component_draw(&mut self, drawer: &mut crate::ComponentDrawer) {
        self.component_area = drawer.area;
    }
}
