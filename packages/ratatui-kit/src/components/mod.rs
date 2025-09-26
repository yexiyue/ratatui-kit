// 适配器组件，用于桥接外部 widget 或自定义渲染逻辑。
mod adapter;
pub use adapter::*;
// Fragment 透明容器组件，无额外布局节点，常用于包裹多个子元素。
mod fragment;
pub use fragment::*;
// 视图容器组件，支持布局、嵌套、样式等，常用于包裹和组织子组件。
mod view;
pub use view::*;
// 边框组件，为内容添加可定制的边框和标题。
mod border;
pub use border::*;
// 模态框组件，支持弹窗、遮罩等交互场景。
mod modal;
pub use modal::*;
// 滚动视图组件，支持内容滚动，适合长列表、文档阅读等。
pub mod scroll_view;
pub use scroll_view::*;
// 上下文提供者组件，实现依赖注入和全局状态共享。
mod context_provider;
pub use context_provider::*;

#[cfg(feature = "textarea")]
// 多行文本输入组件，支持光标、占位符、行号等，适合编辑器、表单等场景。
mod textarea;
#[cfg(feature = "textarea")]
pub use textarea::*;

#[cfg(feature = "router")]
// 路由组件，支持页面跳转、参数、嵌套路由等，适合多页面终端应用。
mod router;
#[cfg(feature = "router")]
pub use router::*;
