//! Offscreen-rendering test helpers (feature `test-util`).
//!
//! Renders a component tree once (or over several frames, for components
//! that read layout info from the previous frame, e.g. `Input`'s
//! `use_previous_size`) into a [`ratatui::buffer::Buffer`] without a real
//! terminal. Intended for `ratatui-kit-<name>` extension crates that want to
//! write the same kind of integration test the core crate uses internally in
//! `render/harness.rs` — e.g. mounting a component under a real
//! `PaletteProvider` and asserting the rendered cell styles change when the
//! `Palette` changes, rather than only unit-testing `ComponentTheme::from_palette`
//! in isolation.
//!
//! ```
//! use ratatui_kit::prelude::*;
//! use ratatui_kit::test_util::render_frame;
//!
//! let mut palette = Palette::default();
//! palette.fg = ratatui::style::Color::Red;
//! let buf = render_frame(
//!     element!(PaletteProvider(palette: palette) { Text(text: "hi") }),
//!     8,
//!     1,
//! );
//! assert_eq!(buf[(0, 0)].style().fg, Some(ratatui::style::Color::Red));
//! ```

use std::io;

use ratatui::{backend::TestBackend, buffer::Buffer};

use crate::{
    AnyElement, ComponentDrawer, element::ElementRepr, render::tree::Tree,
    terminal::UpdaterTerminal,
};

// no-op 终端：`insert_before` 空操作，仅供驱动 update；事件不经终端订阅。
struct NoopTerminal;

impl UpdaterTerminal for NoopTerminal {
    fn insert_before(
        &mut self,
        _height: u16,
        _draw_fn: Box<dyn FnOnce(&mut Buffer)>,
    ) -> io::Result<()> {
        Ok(())
    }
}

/// Render a component tree once into an offscreen buffer.
#[must_use]
pub fn render_frame(el: impl Into<AnyElement<'static>>, width: u16, height: u16) -> Buffer {
    render_frames(el, width, height, 1)
}

/// Render a component tree over several frames into an offscreen buffer.
/// Needed for components that read layout info from the previous frame
/// (e.g. `Input`'s `use_previous_size`).
#[must_use]
pub fn render_frames(
    el: impl Into<AnyElement<'static>>,
    width: u16,
    height: u16,
    frames: usize,
) -> Buffer {
    let mut el = el.into();
    let helper = el.helper();
    let mut tree = Tree::new(el.props_mut(), helper);

    let mut noop = NoopTerminal;
    let mut terminal = ratatui::Terminal::new(TestBackend::new(width, height)).unwrap();

    for _ in 0..frames.max(1) {
        tree.update_once(&mut noop);
        terminal
            .draw(|frame| {
                let area = frame.area();
                let mut drawer = ComponentDrawer::new(frame, area);
                tree.draw_root(&mut drawer);
            })
            .unwrap();
    }

    terminal.backend().buffer().clone()
}
