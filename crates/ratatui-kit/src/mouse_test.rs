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
        // 测试专用:leak 使元素获得 'static,Tree 借用随之 'static(测试进程生命周期,可接受)。
        let el: &'static mut AnyElement<'static> = Box::leak(Box::new(el.into()));
        let helper = el.helper();
        let props = el.props_mut();
        let mut this = Self {
            tree: Tree::new_with_mouse(props, helper, mouse),
            terminal: ratatui::Terminal::new(TestBackend::new(40, 10)).unwrap(),
            noop: NoopUpdater,
        };
        this.frame();
        this
    }

    fn frame(&mut self) {
        self.tree.update_once(&mut self.noop);
        self.terminal
            .draw(|frame| {
                let area = frame.area();
                let mut drawer = ComponentDrawer::new(frame, area);
                self.tree.draw_root(&mut drawer);
            })
            .unwrap();
    }

    // 派发一个鼠标事件(模拟渲染循环:dispatch 后必重绘)。
    pub(crate) fn dispatch(&mut self, kind: MouseEventKind, col: u16, row: u16) {
        self.tree
            .system_context_mut()
            .input
            .dispatch(Event::Mouse(MouseEvent {
                kind,
                column: col,
                row,
                modifiers: KeyModifiers::NONE,
            }));
        self.frame();
    }

    // 当前离屏渲染结果(断言用)。
    pub(crate) fn buffer(&mut self) -> Buffer {
        self.terminal.backend().buffer().clone()
    }
}
