use crate::{Component, Props, State};
use ratatui::widgets::StatefulWidget;

// 与 `widget.rs` 同理:0.30 起含 `Block` 的 stateful widget(如 `List`)不再 Send + Sync。
// 此处放宽对被适配 widget `T` 自身的 Send + Sync 要求,改为对适配器及其 props 以
// `unsafe impl` 断言(见 `widget.rs` 顶部安全说明)。`T::State` 仍要求 Send + Sync,
// 因 `State<T::State>` 需可存储于框架的(Send)状态体系中;0.30 起 `StatefulWidget::State`
// 去掉了隐式 `Sized`,故显式补 `Sized`。

pub struct StatefulWidgetAdapterProps<T>
where
    T: StatefulWidget + 'static,
    T::State: Sized + Sync + Send + 'static,
{
    pub inner: T,
    pub state: State<T::State>,
}

// Safety: 见 `widget.rs` 顶部说明。
unsafe impl<T> Send for StatefulWidgetAdapterProps<T>
where
    T: StatefulWidget + 'static,
    T::State: Sized + Sync + Send + 'static,
{
}
unsafe impl<T> Sync for StatefulWidgetAdapterProps<T>
where
    T: StatefulWidget + 'static,
    T::State: Sized + Sync + Send + 'static,
{
}
unsafe impl<T> Props for StatefulWidgetAdapterProps<T>
where
    T: StatefulWidget + 'static,
    T::State: Sized + Sync + Send + 'static,
{
}

pub struct StatefulWidgetAdapter<T>
where
    T: StatefulWidget + 'static,
    T::State: Sized + Sync + Send + 'static,
{
    inner: T,
    state: State<T::State>,
}

// Safety: 见 `widget.rs` 顶部说明。
unsafe impl<T> Send for StatefulWidgetAdapter<T>
where
    T: StatefulWidget + 'static,
    T::State: Sized + Sync + Send + 'static,
{
}
unsafe impl<T> Sync for StatefulWidgetAdapter<T>
where
    T: StatefulWidget + 'static,
    T::State: Sized + Sync + Send + 'static,
{
}

impl<T> Component for StatefulWidgetAdapter<T>
where
    T: StatefulWidget + 'static + Unpin + Clone,
    T::State: Sized + Sync + Send + 'static + Unpin,
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
