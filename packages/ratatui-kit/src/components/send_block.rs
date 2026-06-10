//! [`SendBlock`]:`Option<Block>` 的 `Send + Sync` 包装。
//!
//! ratatui 0.30 起 `Block` 内含 `Arc<dyn CellEffect>`(自定义阴影效果的类型擦除句柄)
//! 而不再是 `Send + Sync`。但 ratatui-kit 的 [`Props`](crate::Props) /
//! [`Component`](crate::Component) 均要求 `Send + Sync`——组件的 `wait()` 经
//! `BoxFuture`(Send)轮询(见 `render/tree.rs`),`use_future` 也要求 `F: Send`。
//! 框架已对类型擦除后的 props 统一以 `unsafe impl Send/Sync for AnyProps` 承担线程安全
//! (见 `props.rs`),本包装沿用同一设计,使含 `Block` 的 props/组件重新满足 `Send + Sync`。

use ratatui::widgets::Block;
use std::ops::{Deref, DerefMut};

/// `Option<Block>` 的 `Send + Sync` 包装,供组件 props/字段承载可选边框。
///
/// 通过 [`Deref`] 暴露内部 `Option<Block>`(故 `.is_some()`/`.as_ref()` 等照常可用),
/// 并提供 `From<Block>` 与 `From<Option<Block>>`;配合 `element!` 宏对字段值自动 `.into()`,
/// 书写方式与原 `Option<Block<'static>>` 字段完全一致(如 `block: Block::bordered()...`)。
#[derive(Default, Clone, Debug)]
pub struct SendBlock(pub Option<Block<'static>>);

// Safety: 见模块文档。ratatui-kit/上层应用构造的 `Block` 不挂自定义阴影效果
// (`Effect` 为 `Overlay`/`Symbol`,不含 `Arc<dyn CellEffect>`),内部 `Block` 实际即为
// `Send + Sync`;且组件树仅在单线程渲染路径中访问,断言成立。
unsafe impl Send for SendBlock {}
unsafe impl Sync for SendBlock {}

impl Deref for SendBlock {
    type Target = Option<Block<'static>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SendBlock {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Block<'static>> for SendBlock {
    fn from(block: Block<'static>) -> Self {
        Self(Some(block))
    }
}

impl From<Option<Block<'static>>> for SendBlock {
    fn from(block: Option<Block<'static>>) -> Self {
        Self(block)
    }
}
