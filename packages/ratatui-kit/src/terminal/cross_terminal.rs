use super::TerminalImpl;
use crossterm::event::{self, EventStream};
use futures::{StreamExt, stream::BoxStream};
use ratatui::{Frame, TerminalOptions};
use std::io::{self};

// ================== 终端核心功能实现 ==================

// 跨平台终端结构体
// input_is_terminal: 标记标准输入是否为终端设备
// dest: 标准输出流（用于终端操作）
// raw_mode_enabled: 原始模式启用状态
// enabled_keyboard_enhancement: 键盘增强功能状态
// fullscreen: 是否启用全屏模式
pub struct CrossTerminal {
    terminal: ratatui::DefaultTerminal,
}

impl CrossTerminal {
    // 创建终端实例
    // fullscreen: 是否启用备用屏幕（全屏模式）
    pub fn new() -> io::Result<Self> {
        Ok(Self {
            terminal: ratatui::init(),
        })
    }

    // 启用/禁用原始模式
    pub fn with_options(options: TerminalOptions) -> io::Result<Self> {
        Ok(Self {
            terminal: ratatui::init_with_options(options),
        })
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
}
