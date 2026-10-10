use super::TerminalImpl;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, EventStream},
    execute,
};
use futures::{StreamExt, stream::BoxStream};
use ratatui::{Frame, TerminalOptions};
use std::io::{self, Write};

// ================== 终端核心功能实现 ==================

// 跨平台终端结构体
pub struct CrossTerminal {
    terminal: ratatui::DefaultTerminal,
}

impl CrossTerminal {
    // 创建终端实例
    pub fn new() -> io::Result<Self> {
        Ok(Self {
            terminal: ratatui::init(),
        })
    }

    // 创建终端实例;`mouse: true` 时额外执行鼠标捕获(指针事件投递的前提)。
    pub fn new_with_mouse(mouse: bool) -> io::Result<Self> {
        let this = Self::new()?;
        if mouse {
            set_mouse_capture(true)?;
        }
        Ok(this)
    }

    // 启用/禁用原始模式
    pub fn with_options(options: TerminalOptions) -> io::Result<Self> {
        Ok(Self {
            terminal: ratatui::init_with_options(options),
        })
    }

    // 自定义 Viewport 创建终端;`mouse: true` 时额外执行鼠标捕获。
    pub fn with_options_and_mouse(options: TerminalOptions, mouse: bool) -> io::Result<Self> {
        let this = Self::with_options(options)?;
        if mouse {
            set_mouse_capture(true)?;
        }
        Ok(this)
    }
}

// 执行/解除终端鼠标捕获。退出恢复由 `RestoreGuard` 经 `set_mouse_capture(false)` 完成。
pub(crate) fn set_mouse_capture(enabled: bool) -> io::Result<()> {
    if enabled {
        execute!(io::stdout(), EnableMouseCapture)
    } else {
        execute!(io::stdout(), DisableMouseCapture)
    }
}

// ================== 生命周期管理 ==================
// 不能在这里调用restore，因为render_loop中途可能会panic，导致提前drop，所以报错信息会被覆盖
// 需要在render_loop结束后手动调用restore
// impl Drop for CrossTerminal {
//     // 析构函数：自动恢复终端原始状态
//     fn drop(&mut self) {
//         ratatui::restore();
//     }
// }

// ================== 终端接口实现 ==================

impl TerminalImpl for CrossTerminal {
    type Event = event::Event;

    // 创建事件流
    fn event_stream(&mut self) -> io::Result<BoxStream<'static, Self::Event>> {
        // 创建事件流并过滤错误
        Ok(EventStream::new()
            .filter_map(|event| async move { event.ok() })
            .boxed())
    }

    // 检测Ctrl+C组合键
    fn received_ctrl_c(event: Self::Event) -> bool {
        matches!(
            event,
            event::Event::Key(event::KeyEvent {
                code: event::KeyCode::Char('c'),
                modifiers: event::KeyModifiers::CONTROL,
                kind: event::KeyEventKind::Press,
                ..
            })
        )
    }

    fn draw<F>(&mut self, f: F) -> io::Result<()>
    where
        F: FnOnce(&mut Frame),
    {
        self.terminal.draw(f)?;
        Ok(())
    }

    fn insert_before<F>(&mut self, height: u16, draw_fn: F) -> io::Result<()>
    where
        F: FnOnce(&mut ratatui::prelude::Buffer),
    {
        self.terminal.insert_before(height, draw_fn)?;
        Ok(())
    }

    // 惰性升级为 any-motion 上报(1003h):悬停追踪需要「无按键移动」事件。
    // crossterm 的 EnableMouseCapture 默认只开 1002h(按键拖动期间上报移动),
    // 1003h 由本框架在存在 hover/Move 追踪注册时按需补开,只升不降。
    fn enable_any_motion(&mut self) -> io::Result<()> {
        let mut out = io::stdout();
        out.write_all(b"\x1b[?1003h")?;
        out.flush()
    }
}
