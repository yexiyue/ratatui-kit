---
title: "组件"
sidebar:
    order: 4
---

Ratatui Kit 提供了丰富的常用组件，帮助你高效构建终端 UI：

- `View`：基础容器组件
- `ScrollView`：可滚动容器组件
- `Border`：边框容器组件
- `Fragment`：片段组件，用于组合其他组件
- `ContextProvider`：上下文提供者组件，用于在组件树中传递数据
- `Modal`：模态对话框组件
- `TextArea`：文本输入区域组件
- `RouterProvider`：路由提供者组件，用于处理路由和导航
- `Outlet`：路由出口组件，用于渲染当前路由对应的组件

## 自定义组件

Ratatui Kit 支持灵活的自定义组件能力，主要有两种方式：

### 1. 使用宏快速定义组件

适用于大多数简单场景。你只需定义属性结构体（如有需要），并用 `#[component]` 宏修饰组件函数，即可快速组合和复用已有组件。

下面是一个典型的用法示例：

```rust
#[derive(Props)]
struct MyComponentProps {
    // 定义组件的属性
}

#[component]
fn MyComponent(props: MyComponentProps,hooks:Hooks) -> impl Into<AnyElement<'static>> {
    // 组件的实现
    element!(
        // ...
    )
}
```

### 2. 实现 Component trait（进阶用法）

当你需要更细致地控制组件的生命周期、状态和渲染逻辑时，可以手动实现 `Component` trait。这样可以实现高度自定义的复杂组件。

`Component` trait 的主要接口如下，建议仅在有特殊需求时使用：

```rust
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

    fn draw(&mut self, drawer: &mut ComponentDrawer<'_, '_>) {
        self.render_ref(drawer.area, drawer.buffer_mut());
    }

    // 默认使用flex布局计算子组件的area
    fn calc_children_areas(
        &self,
        children: &Components,
        layout_style: &LayoutStyle,
        drawer: &mut ComponentDrawer<'_, '_>,
    ) -> Vec<ratatui::prelude::Rect> {
        // ...
    }

    fn poll_change(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> std::task::Poll<()> {
        std::task::Poll::Pending
    }

    fn render_ref(&self, _area: ratatui::layout::Rect, _buf: &mut ratatui::buffer::Buffer) {}
}
```

### 自定义 Border 组件示例

以下以 Border 组件为例，详细展示自定义组件的完整流程。每一步都配有代码和简要说明，便于理解：

1. 定义属性结构体：用于描述组件的所有可配置项，包括边框样式、内边距、标题等。

```rust
#[with_layout_style]
#[derive(Props)]
/// Border 组件属性。
pub struct BorderProps<'a> {
    /// 内边距。
    pub padding: Padding,
    /// 边框样式。
    pub border_style: ratatui::style::Style,
    /// 显示哪些边。
    pub borders: ratatui::widgets::Borders,
    /// 边框字符集。
    pub border_set: border::Set,
    /// 整体样式。
    pub style: ratatui::style::Style,
    /// 子元素列表。
    pub children: Vec<AnyElement<'a>>,
    /// 顶部标题。
    pub top_title: Option<Line<'static>>,
    /// 底部标题。
    pub bottom_title: Option<Line<'static>>,
}
```

注意到我们使用了`#[with_layout_style]`宏来自动为组件添加布局样式支持。
它会为`BorderProps`添加`LayoutStyle`相关的字段，并为`BorderProps`实现`layout_style`方法，方便我们从props中构造`LayoutStyle`。

2. 定义组件结构体：包含组件运行时所需的字段。

```rust
pub struct Border {
    pub padding: Padding,
    pub border_style: ratatui::style::Style,
    pub borders: ratatui::widgets::Borders,
    pub border_set: border::Set,
    pub style: ratatui::style::Style,
    pub top_title: Option<Line<'static>>,
    pub bottom_title: Option<Line<'static>>,
}
```

这个Border结构体包含一些必要的字段，用于稍后实现渲染方法。

3. 实现 Component trait：实现组件的生命周期方法，包括创建、更新和渲染。

```rust
impl Component for Border {
    type Props<'a> = BorderProps<'a>;

    /// 根据属性创建 Border 组件实例
    fn new(props: &Self::Props<'_>) -> Self {
        Self {
            padding: props.padding,
            border_style: props.border_style,
            borders: props.borders,
            border_set: props.border_set,
            style: props.style,
            top_title: props.top_title.clone(),
            bottom_title: props.bottom_title.clone(),
        }
    }

    /// 根据最新属性和子组件更新自身状态
    fn update(
        &mut self,
        props: &mut Self::Props<'_>,
        _hooks: crate::Hooks,
        updater: &mut crate::ComponentUpdater,
    ) {
        // 获取布局属性
        let layout_style = props.layout_style();
        // 用新属性重建自身
        *self = Self {
            padding: props.padding,
            border_style: props.border_style,
            borders: props.borders,
            border_set: props.border_set,
            style: props.style,
            top_title: props.top_title.clone(),
            bottom_title: props.bottom_title.clone(),
        };
        // 设置布局样式
        updater.set_layout_style(layout_style);
        // 更新子组件
        updater.update_children(&mut props.children, None);
    }

    /// 渲染 Border 组件
    fn draw(&mut self, drawer: &mut crate::ComponentDrawer<'_, '_>) {
        // 构建 Block，设置样式、边框、内边距等
        let mut block = Block::new()
            .style(self.style)
            .borders(self.borders)
            .border_set(self.border_set)
            .border_style(self.border_style)
            .padding(self.padding);

        // 设置顶部标题（如有）
        if let Some(top_title) = &self.top_title {
            block = block.title_top(top_title.clone());
        }

        // 设置底部标题（如有）
        if let Some(bottom_title) = &self.bottom_title {
            block = block.title_bottom(bottom_title.clone());
        }

        // 计算内容区域
        let inner_area = block.inner(drawer.area);
        // 渲染边框
        block.render(drawer.area, drawer.buffer_mut());
        // 更新绘制区域为内容区，供子组件使用
        drawer.area = inner_area;
    }
}
```

通过上述步骤，即可实现一个支持属性和布局的自定义组件。你可以根据实际需求，扩展更多属性和逻辑。

通过这两种方式，你可以灵活扩展和组合 Ratatui Kit 的组件体系，满足各种终端 UI 场景需求。建议优先使用宏方式，只有在需要完全自定义行为时再手动实现 trait。如果你想深入了解更多实现细节，欢迎直接阅读 [Ratatui Kit 原理讲解](https://yexiyue.github.io/ratatui-kit-website/principle/01-%E5%89%8D%E8%A8%80/)！
