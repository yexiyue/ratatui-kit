// 适配器组件，用于桥接外部 widget 或自定义渲染逻辑。
pub mod adapter;
pub use adapter::*;
// Fragment 透明容器组件，无额外布局节点，常用于包裹多个子元素。
pub mod fragment;
pub use fragment::*;
// 视图容器组件，支持布局、嵌套、样式等，常用于包裹和组织子组件。
pub mod view;
pub use view::*;
// 边框组件，为内容添加可定制的边框和标题。
pub mod border;
pub use border::*;
// 模态框组件，支持弹窗、遮罩等交互场景。
pub mod modal;
pub use modal::*;
// 滚动视图组件，支持内容滚动，适合长列表、文档阅读等。
pub mod scroll_view;
pub use scroll_view::*;
// 上下文提供者组件，实现依赖注入和全局状态共享。
mod context_provider;
pub use context_provider::*;
// 中心布局组件，用于居中布局，适合内容居中显示。
pub mod center;
pub use center::*;

// 文本组件，用于显示文本内容，支持样式、超链接等。
pub mod text;
pub use text::*;

// 定位组件，支持绝对定位，适合复杂布局需求。
pub mod positioned;
pub use positioned::*;

#[cfg(feature = "input")]
pub mod input;
#[cfg(feature = "input")]
pub use input::*;
#[cfg(feature = "input")]
pub use tui_input;

#[cfg(feature = "tree")]
pub mod tree_select;
#[cfg(feature = "tree")]
pub use tree_select::*;
#[cfg(feature = "tree")]
pub use tui_tree_widget;

#[cfg(feature = "textarea")]
// 多行文本输入组件，支持光标、占位符、行号等，适合编辑器、表单等场景。
pub mod textarea;
#[cfg(feature = "textarea")]
pub use textarea::*;

#[cfg(feature = "router")]
// 路由组件，支持页面跳转、参数、嵌套路由等，适合多页面终端应用。
pub mod router;
#[cfg(feature = "router")]
pub use router::*;
