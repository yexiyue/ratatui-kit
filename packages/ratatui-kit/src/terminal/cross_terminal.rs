use super::TerminalImpl;
use crossterm::{
    cursor,
    event::{self, EventStream, KeyboardEnhancementFlags},
    execute, queue, terminal,
};
use futures::{StreamExt, stream::BoxStream};
use ratatui::Frame;
use std::io::{self, IsTerminal, stdout};
fn set_panic_hook() {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        terminal::disable_raw_mode().unwrap();
        execute!(
            stdout(),
            terminal::LeaveAlternateScreen,
            event::DisableMouseCapture,
            event::PopKeyboardEnhancementFlags,
            cursor::Show
        )
        .unwrap();
        hook(info);
    }));
}

// ================== 终端核心功能实现 ==================

// 跨平台终端结构体
// input_is_terminal: 标记标准输入是否为终端设备
// dest: 标准输出流（用于终端操作）
// raw_mode_enabled: 原始模式启用状态
// enabled_keyboard_enhancement: 键盘增强功能状态
// fullscreen: 是否启用全屏模式
pub struct CrossTerminal {
    input_is_terminal: bool,
    dest: std::io::Stdout,
    raw_mode_enabled: bool,
    enabled_keyboard_enhancement: bool,
    fullscreen: bool,
    terminal: ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
}

impl CrossTerminal {
    // 创建终端实例
    // fullscreen: 是否启用备用屏幕（全屏模式）
    pub fn new(fullscreen: bool) -> io::Result<Self> {
        let mut dest = io::stdout();
        // 隐藏光标
        queue!(dest, cursor::Hide)?;

        // 进入备用屏幕（全屏模式）
        if fullscreen {
            queue!(dest, terminal::EnterAlternateScreen)?;
        }

        // 设置panic钩子，确保异常时恢复终端状态
        set_panic_hook();

        Ok(Self {
            input_is_terminal: io::stdin().is_terminal(),
            raw_mode_enabled: false,
            enabled_keyboard_enhancement: false,
            fullscreen,
            terminal: ratatui::Terminal::new(ratatui::backend::CrosstermBackend::new(stdout()))?,
            dest,
        })
    }

    // 启用/禁用原始模式
    // enabled: 目标模式状态
    pub fn set_raw_mode_enabled(&mut self, enabled: bool) -> io::Result<()> {
        if enabled != self.raw_mode_enabled {
            if enabled {
                // 支持键盘增强时启用
                if terminal::supports_keyboard_enhancement().unwrap_or(false) {
                    execute!(
                        self.dest,
                        event::PushKeyboardEnhancementFlags(
                            KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                        )
                    )?;
                    self.enabled_keyboard_enhancement = true;
                }
                // 全屏模式下启用鼠标捕获
                if self.fullscreen {
                    execute!(self.dest, event::EnableMouseCapture)?;
                }

                // 启用原始模式
                terminal::enable_raw_mode()?;
            } else {
                // 禁用原始模式
                terminal::disable_raw_mode()?;
                // 恢复键盘增强状态
                if self.enabled_keyboard_enhancement {
                    execute!(self.dest, event::PopKeyboardEnhancementFlags)?;
                    self.enabled_keyboard_enhancement = false;
                }
                // 禁用鼠标捕获
                if self.fullscreen {
                    execute!(self.dest, event::DisableMouseCapture)?;
                }
            }

            self.raw_mode_enabled = enabled;
        }

        Ok(())
    }
}

// ================== 生命周期管理 ==================

impl Drop for CrossTerminal {
    // 析构函数：自动恢复终端原始状态
    fn drop(&mut self) {
        let _ = self.set_raw_mode_enabled(false);
        if self.fullscreen {
            let _ = queue!(self.dest, terminal::LeaveAlternateScreen);
        }
        let _ = execute!(self.dest, cursor::Show);
    }
}

// ================== 终端接口实现 ==================

impl TerminalImpl for CrossTerminal {
    type Event = event::Event;

    // 查询原始模式状态
    fn is_raw_mode_enabled(&self) -> bool {
        self.raw_mode_enabled
    }

    // 创建事件流
    fn event_stream(&mut self) -> io::Result<BoxStream<'static, Self::Event>> {
        if !self.input_is_terminal {
            return Ok(futures::stream::pending().boxed());
        }

        // 确保进入原始模式
        self.set_raw_mode_enabled(true)?;

        // 创建事件流并过滤错误
        Ok(EventStream::new()
            .filter_map(|event| async move {
                match event {
                    Ok(event) => Some(event),
                    Err(_) => None, // 忽略无效事件
                }
            })
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
}
