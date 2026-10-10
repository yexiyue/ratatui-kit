use super::TerminalImpl;
use crossterm::{
    Command,
    event::{self, DisableMouseCapture, EventStream},
    execute,
};
use futures::{StreamExt, stream::BoxStream};
use ratatui::{Frame, TerminalOptions};
use std::{
    fmt,
    io::{self, Write},
};

// crossterm 的 EnableMouseCapture 同时开启 1003h。这里仅请求按压和拖动上报,
// 无按键移动由 enable_any_motion 在存在 hover/Move 追踪时单独开启。
struct EnableButtonMouseCapture;

impl Command for EnableButtonMouseCapture {
    fn write_ansi(&self, f: &mut impl fmt::Write) -> fmt::Result {
        f.write_str("\x1b[?1000h\x1b[?1002h\x1b[?1015h\x1b[?1006h")
    }

    #[cfg(windows)]
    fn execute_winapi(&self) -> io::Result<()> {
        event::EnableMouseCapture.execute_winapi()
    }

    #[cfg(windows)]
    fn is_ansi_code_supported(&self) -> bool {
        event::EnableMouseCapture.is_ansi_code_supported()
    }
}

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
        execute!(io::stdout(), EnableButtonMouseCapture)
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
    // 初始捕获仅请求按键/拖动,1003h 在存在 hover/Move 追踪时按需补开,只升不降。
    fn enable_any_motion(&mut self) -> io::Result<()> {
        let mut out = io::stdout();
        out.write_all(b"\x1b[?1003h")?;
        out.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_capture_reports_buttons_and_drags_without_all_motion() {
        let mut ansi = String::new();
        EnableButtonMouseCapture.write_ansi(&mut ansi).unwrap();
        assert!(ansi.contains("\x1b[?1000h"));
        assert!(ansi.contains("\x1b[?1002h"));
        assert!(ansi.contains("\x1b[?1006h"));
        assert!(!ansi.contains("\x1b[?1003h"));
    }
}
