use crate::{Component, State};
use ratatui::widgets::StatefulWidget;
use ratatui_kit_macros::Props;

#[derive(Props)]
pub struct StatefulWidgetAdapterProps<T>
where
    T: StatefulWidget + Sync + Send + 'static,
    T::State: Sync + Send + 'static,
{
    pub inner: T,
    pub state: State<T::State>,
}

pub struct StatefulWidgetAdapter<T>
where
    T: StatefulWidget + Sync + Send + 'static,
    T::State: Sync + Send + 'static,
{
    inner: T,
    state: State<T::State>,
}

impl<T> Component for StatefulWidgetAdapter<T>
where
    T: StatefulWidget + Sync + Send + 'static + Unpin + Clone,
    T::State: Sync + Send + 'static + Unpin,
{
    type Props<'a> = StatefulWidgetAdapterProps<T>;

    fn new(props: &Self::Props<'_>) -> Self {
        Self {
            inner: props.inner.clone(),
            state: props.state,
        }
    }

    fn update(
        &mut self,
        props: &mut Self::Props<'_>,
        _hooks: crate::Hooks,
        _updater: &mut crate::ComponentUpdater,
    ) {
        self.inner = props.inner.clone();
        self.state = props.state;
    }

    fn draw(&mut self, drawer: &mut crate::ComponentDrawer<'_, '_>) {
        drawer.render_stateful_widget(
            self.inner.clone(),
            drawer.area,
            &mut self.state.write_no_update(),
        );
    }
}
