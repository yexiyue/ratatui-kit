use crate::Component;
use ratatui::widgets::WidgetRef;
use ratatui_kit_macros::Props;
use std::sync::Arc;

#[derive(Props, Default)]
pub struct AdapterProps(pub Option<Arc<dyn WidgetRef + Sync + Send + 'static>>);

pub struct Adapter {
    inner: Arc<dyn WidgetRef + Sync + Send + 'static>,
}
impl Component for Adapter {
    type Props<'a> = AdapterProps;

    fn new(props: &Self::Props<'_>) -> Self {
        Self {
            inner: props
                .0
                .as_ref()
                .expect("AdapterProps must contain a WidgetRef")
                .clone(),
        }
    }

    fn update(
        &mut self,
        props: &mut Self::Props<'_>,
        _hooks: crate::Hooks,
        _updater: &mut crate::ComponentUpdater,
    ) {
        self.inner = props
            .0
            .as_ref()
            .expect("AdapterProps must contain a WidgetRef")
            .clone();
    }

    fn render_ref(&self, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer) {
        self.inner.render_ref(area, buf);
    }
}
