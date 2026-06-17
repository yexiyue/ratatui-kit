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
    // 约束改为**按值** `Widget`(而非旧的 `for<'a> &'a T: Widget`)。这是一笔**双向取舍**,非纯扩展:
    // - 纳入:0.30 起内置 widget 多同时实现按值与按引用 `Widget`,但不少第三方/纯按值 widget
    //   (如 `tui-big-text` 的 `BigText`)只实现按值 `Widget`,旧约束会把它们挡在 `widget(...)` 之外。
    // - 失去:**只实现按引用 `impl Widget for &T` 的 widget**(如 `ratatui-widgets` 的 `Shadow`——
    //   亦 ratatui 官方为无状态 widget 推荐的写法)在新约束下不再被接纳;如需用可自行 wrap 成按值 widget。
    // 组件已要求 `Clone`(new/update 从借用 props 拷入),draw 里再 clone 一次按值渲染,代价对 TUI 可忽略。
    T: 'static + Unpin + Clone + Widget,
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
        // 按值渲染:克隆持久持有的 widget 交给消费式 `Widget::render`。
        drawer.render_widget(self.inner.clone(), drawer.area);
    }
}
