use crate::{
    element::ElementType,
    hooks::Hooks,
    layout_style::LayoutStyle,
    props::{AnyProps, Props},
    render::{ComponentDrawer, ComponentUpdater},
};
use std::{any::Any, pin::Pin, task::Context};

mod component_helper;
pub(crate) use component_helper::{ComponentHelper, ComponentHelperExt};

mod instantiated_component;
pub use instantiated_component::{Components, InstantiatedComponent};
use ratatui::layout::{Direction, Layout};

/// 组件系统核心 trait，所有自定义 UI 组件都需实现。
///
/// - 通过关联类型 `Props` 定义属性类型，支持生命周期。
/// - `new` 创建组件实例。
/// - `update` 响应 props/hook 变化，适合副作用、事件注册等。
/// - `draw` 渲染组件内容。
/// - `calc_children_areas` 默认 flex 布局计算子组件区域，可重写自定义布局。
/// - `poll_change` 支持异步/响应式副作用。
/// - `render_ref` 低级渲染接口，通常无需重写。
///
/// # 手动实现 Component 示例
///
/// ```rust
/// use ratatui_kit::prelude::*;
/// use ratatui::{style::Style, text::Line};
///
/// pub struct MyCounter;
///
/// impl Component for MyCounter {
///     type Props<'a> = NoProps;
///     fn new(_props: &Self::Props<'_>) -> Self {
///         Self
///     }
///     fn update(
///         &mut self,
///         _props: &mut Self::Props<'_>,
///         mut hooks: Hooks,
///         _updater: &mut ComponentUpdater,
///     ) {
///         let mut state = hooks.use_state(|| 0);
///         hooks.use_events(move |event| {
///             // 事件处理逻辑
///         });
///         // ...
///     }
///     fn draw(&mut self, drawer: &mut ComponentDrawer<'_, '_>) {
///         let area = drawer.area;
///         let buf = drawer.buffer_mut();
///         Line::styled(format!("Counter: {}", 42), Style::default()).render(area, buf);
///     }
/// }
/// ```
///
/// > 一般用户无需手动实现，推荐使用 `#[component]` 宏自动生成。
pub trait Component: Any + Send + Sync + Unpin {
    type Props<'a>: Props
    where
        Self: 'a;

    fn new(props: &Self::Props<'_>) -> Self;

    fn update(
        &mut self,
        _props: &mut Self::Props<'_>,
        _hooks: Hooks,
        _updater: &mut ComponentUpdater,
    ) {
    }

    fn draw(&mut self, _drawer: &mut ComponentDrawer<'_, '_>) {}

    // 默认使用flex布局计算子组件的area
    fn calc_children_areas(
        &self,
        children: &Components,
        layout_style: &LayoutStyle,
        drawer: &mut ComponentDrawer<'_, '_>,
    ) -> Vec<ratatui::prelude::Rect> {
        let layout = layout_style
            .get_layout()
            .constraints(children.get_constraints(layout_style.flex_direction));

        let areas = layout.split(drawer.area);

        let mut children_areas: Vec<ratatui::prelude::Rect> = vec![];

        let rev_direction = match layout_style.flex_direction {
            Direction::Horizontal => Direction::Vertical,
            Direction::Vertical => Direction::Horizontal,
        };
        for (area, constraint) in areas.iter().zip(children.get_constraints(rev_direction)) {
            let area = Layout::new(rev_direction, [constraint]).split(*area)[0];
            children_areas.push(area);
        }

        children_areas
    }

    fn poll_change(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> std::task::Poll<()> {
        std::task::Poll::Pending
    }
}

pub trait AnyComponent: Any + Send + Sync + Unpin {
    fn update(&mut self, props: AnyProps, hooks: Hooks, updater: &mut ComponentUpdater);

    fn draw(&mut self, drawer: &mut ComponentDrawer);

    fn calc_children_areas(
        &self,
        children: &Components,
        layout_style: &LayoutStyle,
        drawer: &mut ComponentDrawer,
    ) -> Vec<ratatui::prelude::Rect>;

    fn poll_change(self: Pin<&mut Self>, cx: &mut Context) -> std::task::Poll<()>;
}

impl<C> ElementType for C
where
    C: Component,
{
    type Props<'a> = C::Props<'a>;
}

impl<C> AnyComponent for C
where
    C: Any + Component,
{
    fn update(&mut self, mut props: AnyProps, hooks: Hooks, updater: &mut ComponentUpdater) {
        Component::update(
            self,
            unsafe { props.downcast_mut_unchecked() },
            hooks,
            updater,
        );
    }

    fn draw(&mut self, drawer: &mut ComponentDrawer) {
        Component::draw(self, drawer);
    }

    fn calc_children_areas(
        &self,
        children: &Components,
        layout_style: &LayoutStyle,
        drawer: &mut ComponentDrawer,
    ) -> Vec<ratatui::prelude::Rect> {
        Component::calc_children_areas(self, children, layout_style, drawer)
    }

    fn poll_change(self: Pin<&mut Self>, cx: &mut Context) -> std::task::Poll<()> {
        Component::poll_change(self, cx)
    }
}
