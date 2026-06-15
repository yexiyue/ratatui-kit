use crate::{Component, Props};
use ratatui::widgets::Widget;

// 0.30 起 `List`/`Paragraph`/`Table`/`Gauge` 等 widget 内含 `Option<Block>`,而 `Block`
// 因新增阴影效果(`Arc<dyn CellEffect>`)不再 Send + Sync。但 ratatui-kit 的 `Props`/
// `Component` 要求 Send + Sync(组件 `wait()` 经 `BoxFuture` 轮询)。
//
// 适配器在此放宽:不再要求被适配 widget 自身 Send + Sync,改为对适配器及其 props 以
// `unsafe impl` 断言 Send + Sync。这与框架对 `AnyProps` 的既有处理(`unsafe impl Send/Sync`)
// 一致——组件树只在单线程渲染路径中访问,widget 不跨线程并发使用,断言成立。
// 好处:任意 ratatui widget 仍可经 `$expr` 直接嵌入元素树,保持 0.29 的使用体验。

pub struct WidgetAdapterProps<T>
where
    T: Widget + 'static,
{
    pub inner: T,
}

// Safety: 见本文件顶部说明。
unsafe impl<T: Widget + 'static> Send for WidgetAdapterProps<T> {}
unsafe impl<T: Widget + 'static> Sync for WidgetAdapterProps<T> {}
unsafe impl<T: Widget + 'static> Props for WidgetAdapterProps<T> {}

pub struct WidgetAdapter<T>
where
    T: Widget + 'static,
{
    inner: T,
}

// Safety: 见本文件顶部说明。
unsafe impl<T: Widget + 'static> Send for WidgetAdapter<T> {}
unsafe impl<T: Widget + 'static> Sync for WidgetAdapter<T> {}

impl<T> Component for WidgetAdapter<T>
where
    T: Widget + 'static + Unpin + Clone,
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
