use futures::{StreamExt, stream::BoxStream};
use ratatui::buffer::Buffer;
use std::{fmt::Debug, io};

mod cross_terminal;
pub use cross_terminal::CrossTerminal;

pub trait TerminalImpl: Send {
    type Event: Clone + Debug;
    fn event_stream(&mut self) -> io::Result<BoxStream<'static, Self::Event>>;
    fn received_ctrl_c(event: Self::Event) -> bool;
    fn draw<F>(&mut self, f: F) -> io::Result<()>
    where
        F: FnOnce(&mut ratatui::Frame);

    fn insert_before<F>(&mut self, height: u16, draw_fn: F) -> io::Result<()>
    where
        F: FnOnce(&mut Buffer);
}

/// 终端封装：纯 raw event source。
///
/// 事件分发已从「发布订阅广播」迁移到中央 `InputRuntime`（见 [`crate::input`]):
/// 渲染循环经 [`Terminal::next_event`] 取单个 raw 事件,先经 [`TerminalImpl::received_ctrl_c`]
/// 判定退出,否则交 `system_context.input.dispatch(event)` 分层投递。
pub struct Terminal<T = CrossTerminal>
where
    T: TerminalImpl,
{
    inner: Box<T>,
    event_stream: BoxStream<'static, T::Event>,
}

impl<T> Terminal<T>
where
    T: TerminalImpl,
{
    pub fn new(inner: T) -> io::Result<Self> {
        let mut inner = Box::new(inner);
        Ok(Self {
            event_stream: inner.event_stream()?,
            inner,
        })
    }

    pub fn draw<F>(&mut self, f: F) -> io::Result<()>
    where
        F: FnOnce(&mut ratatui::Frame),
    {
        self.inner.draw(f)
    }

    pub fn insert_before<F>(&mut self, height: u16, draw_fn: F) -> io::Result<()>
    where
        F: FnOnce(&mut Buffer),
    {
        self.inner.insert_before(height, draw_fn)
    }

    /// 异步等待下一个 raw 事件。`None` 表示事件流结束。
    ///
    /// 不做 ctrl_c 检测(交调用方经 [`TerminalImpl::received_ctrl_c`] 判定)、不广播。
    pub async fn next_event(&mut self) -> Option<T::Event> {
        self.event_stream.next().await
    }
}

/// update 路径所需终端能力的**对象安全**投影。
///
/// `ComponentUpdater` 持 `&mut dyn UpdaterTerminal` 而非具体 `Terminal<CrossTerminal>`——
/// 因 `update_component` 经 `dyn` 分发,`ComponentUpdater` 必须非泛型,故把终端能力擦除成
/// 对象安全 trait。这使 update 能在无头测试里用 no-op 终端驱动(渲染 harness)。
///
/// 事件订阅已移除（改由 `InputRuntime` 中央分发),故此 trait 仅暴露 update 真正用到的 `insert_before`
/// （闭包 box 化,因 `TerminalImpl::insert_before` 泛型闭包不对象安全)。
pub trait UpdaterTerminal {
    fn insert_before(
        &mut self,
        height: u16,
        draw_fn: Box<dyn FnOnce(&mut Buffer)>,
    ) -> io::Result<()>;
}

impl<T> UpdaterTerminal for Terminal<T>
where
    T: TerminalImpl<Event = crossterm::event::Event>,
{
    fn insert_before(
        &mut self,
        height: u16,
        draw_fn: Box<dyn FnOnce(&mut Buffer)>,
    ) -> io::Result<()> {
        // 转发到 Terminal 的固有泛型 insert_before（Box<dyn FnOnce> 本身即 FnOnce）。
        Terminal::insert_before(self, height, draw_fn)
    }
}
