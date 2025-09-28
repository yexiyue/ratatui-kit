use crate::Component;
use ratatui::widgets::Widget;
use ratatui_kit_macros::Props;

#[derive(Props)]
pub struct WidgetAdapterProps<T>
where
    T: Widget + Sync + Send + 'static,
{
    pub inner: T,
}

pub struct WidgetAdapter<T>
where
    T: Widget + Sync + Send + 'static,
{
    inner: T,
}

impl<T> Component for WidgetAdapter<T>
where
    T: Widget + Sync + Send + 'static + Unpin + Clone,
{
    type Props<'a> = WidgetAdapterProps<T>;

    fn new(props: &Self::Props<'_>) -> Self {
        Self {
            inner: props.inner.clone(),
        }
    }

    fn update(
        &mut self,
        props: &mut Self::Props<'_>,
        _hooks: crate::Hooks,
        _updater: &mut crate::ComponentUpdater,
    ) {
        self.inner = props.inner.clone();
    }

    fn draw(&mut self, drawer: &mut crate::ComponentDrawer<'_, '_>) {
        drawer.render_widget(self.inner.clone(), drawer.area);
    }
}
