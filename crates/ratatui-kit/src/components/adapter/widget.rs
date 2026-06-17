use crate::{Component, Props};
use ratatui::widgets::Widget;

pub struct WidgetAdapterProps<T>
where
    T: 'static,
{
    pub inner: T,
}

impl<T: 'static> Props for WidgetAdapterProps<T> {}

pub struct WidgetAdapter<T>
where
    T: 'static,
{
    inner: T,
}

impl<T> Component for WidgetAdapter<T>
where
    T: 'static + Unpin + Clone,
    // 0.30 起所有内置 widget 都实现了 `Widget for &T`(ratatui 官方推荐写法,
    // 见 ratatui-core widget.rs 文档)。借此约束即可在 draw 里按引用渲染。
    for<'a> &'a T: Widget,
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
        // 按引用渲染,免去每帧一次 clone(渲染热路径)。new/update 仍需 Clone 从
        // 借用的 props 把 widget 拷进持久组件,无法省去。
        drawer.render_widget(&self.inner, drawer.area);
    }
}
