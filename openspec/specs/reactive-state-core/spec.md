# reactive-state-core Specification

## Purpose
TBD - created by archiving change harden-audit-findings. Update Purpose after archive.
## Requirements
### Requirement: try_* 非阻塞且不忙等

`State`/`AtomState` 的 `try_read`/`try_write` SHALL 在无法立即获得借用时立即返回 `None`,MUST NOT 自旋重试(`loop`/`continue`)或阻塞当前线程。原先针对 `AlreadyBorrowedMut` 的重试循环 SHALL 被移除。

#### Scenario: 持写守卫时 try_read 立即返回 None
- **WHEN** 已持有某 state 的可变守卫,再调用其 `try_read`
- **THEN** 立即返回 `None`,不进入自旋循环、不占满 CPU、不卡住渲染线程

### Requirement: read/write 重入快速失败

`read()`/`write()` 在无法借用时 SHALL 快速 panic(经 `try_*` 返回 `None` 后 `expect`),而非忙等或死锁。持守卫期间对同一 state 的重入读写属编程错误,SHALL 以 panic 快速暴露而非静默卡死。

#### Scenario: 持守卫重入 read 快速 panic
- **WHEN** 持有某 state 的可变守卫,再调用其 `read()`
- **THEN** 立即 panic 并给出可定位信息,而非忙等卡死 UI、连 ctrl-c 都收不到

### Requirement: State 与 AtomState 共享单一响应式核心

`State` 与 `AtomState` 的读写访问层与 `Display`/`Debug`/`Hash`/`PartialEq`/`PartialOrd`/`Eq`/算术运算符 SHALL 由单一泛型核心提供,二者差异仅限"单 Waker vs 多 Waker"的变更通知策略;两类型 MUST NOT 各自维护逐行重复的实现。

#### Scenario: 行为等价且单一真源
- **WHEN** 对 `State` 与 `AtomState` 执行相同的读/写/比较/格式化/算术操作
- **THEN** 行为一致,且实现来自同一核心(修改一处即对两者同时生效)

### Requirement: 保留 SyncStorage 与 Send+Sync 以支持后台写入

`State` 与 `AtomState` SHALL 继续使用 `SyncStorage` 并保持 `Send + Sync`,以支持在后台 `tokio::spawn` 任务中移动状态句柄并写入。本变更 MUST NOT 改为 `UnsyncStorage`,也 MUST NOT 移除 `Send + Sync`。

#### Scenario: 后台任务可写状态
- **WHEN** 在 `tokio::spawn` 的后台任务中持有 `State`/`AtomState` 句柄并写入
- **THEN** 写入线程安全生效,并经 Waker 唤醒渲染循环重渲(行为与现状一致)

