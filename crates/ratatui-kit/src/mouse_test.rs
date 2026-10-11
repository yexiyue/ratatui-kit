//! `cfg(test)` 专用:鼠标事件驱动的挂载 harness,组件交互测试共享。
//!
//! 模拟渲染循环「事件之间必有重绘」的节奏:dispatch 后必跑一帧 update + draw。

use std::{cell::RefCell, io, rc::Rc};

use crossterm::event::{Event, KeyModifiers, MouseEvent, MouseEventKind};

use crate::{
    AnyElement, ComponentDrawer, ElementRepr, render::tree::Tree, terminal::UpdaterTerminal,
};
use ratatui::{backend::TestBackend, buffer::Buffer};

pub(crate) type TestLog = Rc<RefCell<Vec<String>>>;

pub(crate) fn log_of(entries: &[&str]) -> Vec<String> {
    entries.iter().map(|s| (*s).to_string()).collect()
}

struct NoopUpdater;

impl UpdaterTerminal for NoopUpdater {
    fn insert_before(
        &mut self,
        _height: u16,
        _draw_fn: Box<dyn FnOnce(&mut Buffer)>,
    ) -> io::Result<()> {
        Ok(())
    }
}

pub(crate) struct Mounted {
    tree: Tree<'static>,
    terminal: ratatui::Terminal<TestBackend>,
    noop: NoopUpdater,
}

impl Mounted {
    // 挂载并跑首帧。`mouse: true` 启用 SystemContext 鼠标标志(指针 hook 注册前提)。
    pub(crate) fn mount(el: impl Into<AnyElement<'static>>, mouse: bool) -> Self {
        Self::mount_sized(el, mouse, 40, 10)
    }

    // 同 `mount`,可指定终端尺寸(默认 40x10 放不下较宽内容时用,避免布局溢出
    // 导致标签画在命中区域外)。
    pub(crate) fn mount_sized(
        el: impl Into<AnyElement<'static>>,
        mouse: bool,
        width: u16,
        height: u16,
    ) -> Self {
        // 测试专用:leak 使元素获得 'static,Tree 借用随之 'static(测试进程生命周期,可接受)。
        let el: &'static mut AnyElement<'static> = Box::leak(Box::new(el.into()));
        let helper = el.helper();
        let props = el.props_mut();
        let mut this = Self {
            tree: Tree::new_with_mouse(props, helper, mouse),
            terminal: ratatui::Terminal::new(TestBackend::new(width, height)).unwrap(),
            noop: NoopUpdater,
        };
        this.frame();
        this
    }

    fn frame(&mut self) {
        for _ in 0..16 {
            self.tree.update_once(&mut self.noop);
            let mut redraw = false;
            self.terminal
                .draw(|frame| {
                    let area = frame.area();
                    let mut drawer = ComponentDrawer::new(frame, area);
                    redraw = self.tree.draw_root(&mut drawer);
                })
                .unwrap();
            if !redraw {
                return;
            }
        }
        panic!("pointer layout synchronization did not settle within 16 frames");
    }

    // 派发一个鼠标事件(模拟渲染循环:dispatch 后必重绘)。
    pub(crate) fn dispatch(&mut self, kind: MouseEventKind, col: u16, row: u16) -> bool {
        let redraw = self
            .tree
            .system_context_mut()
            .input
            .dispatch(Event::Mouse(MouseEvent {
                kind,
                column: col,
                row,
                modifiers: KeyModifiers::NONE,
            }));
        self.frame();
        redraw
    }

    pub(crate) fn resize(&mut self, width: u16, height: u16) {
        self.terminal.backend_mut().resize(width, height);
        self.terminal
            .resize(ratatui::layout::Rect::new(0, 0, width, height))
            .unwrap();
        self.tree
            .system_context_mut()
            .input
            .dispatch(Event::Resize(width, height));
        self.frame();
    }

    // 当前离屏渲染结果(断言用)。
    pub(crate) fn buffer(&mut self) -> Buffer {
        self.terminal.backend().buffer().clone()
    }
}

impl Mounted {
    // 派发一个按键事件(KeyEvent::from(code) 即 Press)。
    pub(crate) fn dispatch_key(&mut self, code: crossterm::event::KeyCode) {
        self.tree
            .system_context_mut()
            .input
            .dispatch(Event::Key(code.into()));
        self.frame();
    }
}

// 在缓冲区里查找文本的首个坐标(用于定位点击位置)。
// 按列拼接 symbol 再做子串搜索:仅适用于 ASCII 标签(宽字符会把列与字节错开)。
pub(crate) fn find_text(buf: &Buffer, needle: &str) -> Option<(u16, u16)> {
    let area = buf.area();
    for row in area.top()..area.bottom() {
        for column in area.left()..area.right() {
            let suffix: String = (column..area.right())
                .map(|x| buf[(x, row)].symbol())
                .collect();
            if suffix.starts_with(needle) {
                return Some((column, row));
            }
        }
    }
    None
}
