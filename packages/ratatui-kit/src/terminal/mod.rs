use futures::{Stream, StreamExt, stream::BoxStream};
use ratatui::buffer::Buffer;
use std::{
    collections::VecDeque,
    fmt::Debug,
    io,
    sync::{Arc, Mutex, Weak},
    task::{Poll, Waker},
};

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

// ================== 发布订阅模式核心组件 ==================

// 事件队列内部结构，支持异步唤醒机制
// pending: 待处理事件队列
// waker: 异步任务唤醒器，用于事件到达时唤醒等待的任务
struct TerminalEventsInner<T> {
    pending: VecDeque<T>,
    waker: Option<Waker>,
}

// 事件流封装结构
// inner: 使用Arc+Mutex实现线程安全的事件队列共享
pub struct TerminalEvents<T> {
    inner: Arc<Mutex<TerminalEventsInner<T>>>,
}

// 实现异步Stream接口，支持事件监听
impl<T> Stream for TerminalEvents<T> {
    type Item = T;
    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let mut inner = self.inner.lock().unwrap();
        if let Some(event) = inner.pending.pop_front() {
            Poll::Ready(Some(event)) // 有事件立即返回
        } else {
            inner.waker = Some(cx.waker().clone()); // 无事件时注册唤醒器
            Poll::Pending
        }
    }
}

// ================== 事件分发核心逻辑 ==================

// 异步事件分发器
// subscribers: 订阅者列表（使用Weak指针避免循环引用）
// event_stream: 输入事件流
// received_ctrl_c: Ctrl+C事件标记
pub struct Terminal<T = CrossTerminal>
where
    T: TerminalImpl,
{
    inner: Box<T>,
    event_stream: BoxStream<'static, T::Event>,
    subscribers: Vec<Weak<Mutex<TerminalEventsInner<T::Event>>>>,
    received_ctrl_c: bool,
}

impl<T> Terminal<T>
where
    T: TerminalImpl,
{
    pub fn new(inner: T) -> io::Result<Self> {
        let mut inner = Box::new(inner);
        Ok(Self {
            event_stream: inner.event_stream()?,
            subscribers: Vec::new(),
            received_ctrl_c: false,
            inner,
        })
    }

    pub fn received_ctrl_c(&self) -> bool {
        self.received_ctrl_c
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

    // 事件订阅方法
    pub fn events(&mut self) -> io::Result<TerminalEvents<T::Event>> {
        // 创建新的事件队列实例
        let inner = Arc::new(Mutex::new(TerminalEventsInner {
            pending: VecDeque::new(),
            waker: None,
        }));

        // 添加弱引用订阅者
        self.subscribers.push(Arc::downgrade(&inner));

        Ok(TerminalEvents { inner })
    }

    // 异步事件分发主循环
    pub async fn wait(&mut self) {
        while let Some(event) = self.event_stream.next().await {
            // 检查是否收到Ctrl+C
            self.received_ctrl_c = T::received_ctrl_c(event.clone());
            if self.received_ctrl_c {
                return; // 终止循环
            }

            // 遍历所有订阅者分发事件
            self.subscribers.retain(|subscriber| {
                if let Some(subscriber) = subscriber.upgrade() {
                    let mut subscriber = subscriber.lock().unwrap();
                    // 将事件加入订阅者队列
                    subscriber.pending.push_back(event.clone());

                    // 唤醒订阅者任务
                    if let Some(waker) = subscriber.waker.take() {
                        waker.wake(); // 触发任务继续执行
                    }

                    true // 保留有效订阅者
                } else {
                    false // 移除失效订阅者
                }
            });
        }
    }
}
