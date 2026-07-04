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
// 确认弹窗组件，封装独占输入层与确认/取消键位。
pub mod confirm_modal;
pub use confirm_modal::*;
// 提示弹窗组件，封装独占输入层与关闭键位。
pub mod alert_modal;
pub use alert_modal::*;
// 快捷键帮助弹窗组件，封装独占输入层与关闭键位。
pub mod shortcut_info_modal;
pub use shortcut_info_modal::*;
// 单选列表组件，封装列表状态与键盘选择事件。
mod list_state;
pub mod select;
pub use select::*;
// 多选列表组件，封装多选状态与键盘选择事件。
pub mod multi_select;
pub use multi_select::*;
// 表格组件，支持自绘 grid、换行、响应式列等高级表格能力。
#[cfg(feature = "table")]
pub mod table;
#[cfg(feature = "table")]
pub use table::*;
// Diff 组件，用于对比两个文本版本的差异。
#[cfg(feature = "diff")]
pub mod diff;
#[cfg(feature = "diff")]
pub use diff::*;
// Markdown 组件，解析和渲染 Markdown 文本。
#[cfg(feature = "markdown")]
pub mod markdown;
#[cfg(feature = "markdown")]
pub use markdown::*;
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
// 自动换行文本组件，用于长文档、日志、小说正文等需要把换行高度交给布局的场景。
pub mod wrapped_text;
pub use wrapped_text::*;

// 定位组件，支持绝对定位，适合复杂布局需求。
pub mod positioned;
pub use positioned::*;

// 分割线组件，渲染水平分隔线。
pub mod divider;
pub use divider::*;
// 引用块容器组件，支持嵌套深度和前缀样式。
pub mod blockquote;
pub use blockquote::*;
// 代码块组件，支持行号和语言标签。
pub mod code_block;
pub use code_block::*;

#[cfg(feature = "input")]
pub mod input;
#[cfg(feature = "input")]
pub use input::*;
#[cfg(feature = "input")]
pub mod search_input;
#[cfg(feature = "input")]
pub use search_input::*;
#[cfg(feature = "input")]
pub use tui_input;

#[cfg(feature = "tree")]
pub mod tree_select;
#[cfg(feature = "tree")]
pub use tree_select::*;
#[cfg(feature = "tree")]
pub use tui_tree_widget;

#[cfg(feature = "virtual-list")]
pub mod virtual_list;
#[cfg(feature = "virtual-list")]
pub use tui_widget_list;
#[cfg(feature = "virtual-list")]
pub use virtual_list::*;

// 注:`textarea` 组件暂时下线(其底层 tui-textarea 尚无 ratatui 0.30 兼容版)。
// 源码隔离保留在 `textarea.rs`(未声明为模块),待依赖支持 0.30 后恢复接入。

#[cfg(feature = "router")]
// 路由组件，支持页面跳转、参数、嵌套路由等，适合多页面终端应用。
pub mod router;
#[cfg(feature = "router")]
pub use router::*;
