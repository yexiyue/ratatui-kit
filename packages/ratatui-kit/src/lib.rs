mod component;
pub mod components;
mod context;
mod element;
mod hooks;
mod multimap;
mod props;
mod render;
mod terminal;

mod flatten_export {
    pub use crate::component::*;
    pub use crate::context::*;
    pub use crate::element::*;
    pub use crate::hooks::*;
    pub use crate::props::*;
    pub use crate::render::*;
    pub use crate::terminal::*;
}

pub use flatten_export::*;
pub use ratatui;
pub use ratatui_kit_macros::*;

pub mod prelude {
    pub use crate::components::*;
    pub use crate::flatten_export::*;
    pub use ratatui_kit_macros::*;
}

// 声明当前crate的名称为ratatui_kit
// 这使得其他模块可以通过`use ratatui_kit::...`来访问本模块的内容
// 因此我们可以使用我们自己的macros和属性
extern crate self as ratatui_kit;
