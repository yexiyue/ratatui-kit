## Context

实证：渲染路径无 `tokio::spawn`；`use_future` 在 `poll_change` 内联 `future.as_mut().poll(cx)`；顶层 `fullscreen()/render_loop` future 经 `#[tokio::main]` 的 `block_on` 驱动。**多线程 runtime 的 `block_on` 不要求被驱动 future Send**（只有 `spawn` 的任务要求 Send，而本框架不 spawn）。故 `Component`/`Hook`/`Props` 的 `Send+Sync` 是自加约束，删之无运行时影响。

当前 `Send` 约束链强迫了 23 处 `unsafe impl Send/Sync`（props/context/text/send_block/2 adapter/router）+ `SendBlock`/`TextParagraph` 两个包装。

## Goals / Non-Goals

**Goals:**
- 删除框架级 `Send+Sync` 约束与全部「为绕过 Send」的 unsafe/包装。
- 一致性：可选边框统一回归 `Option<Block<'static>>`。
- 行为零变化。

**Non-Goals:**
- **不动 `State<T>` 的 `Send + Sync`（`SyncStorage`）——这是有意保留的能力**：用户可 `tokio::spawn` 一个后台任务、移动 `State` 句柄并在另一线程更新状态（写入经 SyncStorage 线程安全，唤醒经 Send+Sync 的 `Waker` 跨线程触发渲染循环）。详见决策 6。
- 不引入框架自身的多线程/spawn（渲染树仍单线程、内联 poll）。
- 不动终端泛型（保留 `Terminal<T>` 多后端能力）。

## Decisions

### 决策 1：`use_future` 用 `LocalBoxFuture`，链式去 Send

`F: Future + Send + 'static` → `F: Future + 'static`；`BoxFuture<'static, ()>` → `LocalBoxFuture<'static, ()>`；`.boxed()` → `.boxed_local()`。`Hook: Unpin + Send` → `Unpin`；`Component`/`AnyComponent` 去 `Send + Sync`。

### 决策 2：`Props` 改安全 trait

`pub unsafe trait Props: Send + Sync {}` → `pub trait Props {}`；`#[derive(Props)]` 生成 `impl Props`（去 `unsafe`）；删 `AnyProps` 的 `unsafe impl Send/Sync`。

### 决策 3：删 `SendBlock`，边框字段回归 `Option<Block<'static>>`

删除 `components/send_block.rs` 及模块声明/导出；`tree_select`/`scroll_view` 的 `block: SendBlock` → `Option<Block<'static>>`，`SendBlock::default()` → `None`，内部 `*self.block`/`.as_ref()` 等按 `Option<Block>` 调整。`element!` 的字段 `.into()` + std `From<Block> for Option<Block>` 保证裸 `Block` 写法不变。

### 决策 4：`TextParagraph` 去 unsafe（保留为 ergonomic newtype）

删其 `unsafe impl Send/Sync` 与 `&TextParagraph: Widget` 是否保留视 adapter 而定（adapter 去 unsafe 后 `WidgetAdapter` 仍要求 `for<'a> &'a T: Widget`，故保留 `&TextParagraph: Widget`）。`From<&str>/Line/Text` 转换保留以维持 `Text(text:)`。

### 决策 5：adapter / router / context 去 unsafe

`WidgetAdapter`/`StatefulWidgetAdapter`/`Route`/`Routes`/`SystemContext` 的 `unsafe impl Send/Sync` 直接删；它们持有的 ratatui 类型不再需要 Send。adapter 的 `Props` 实现由 `unsafe impl` 改 `impl`（Props 已是安全 trait）。

### 决策 6：**保留 `State<T>: Send + Sync`（`SyncStorage`）——后台 spawn 更新状态的刚需

`State` 的 Send 与组件树的 Send 是**两件独立的事**：用户 `tokio::spawn` 移动的是 `State` 句柄（`Copy` 的 box handle），不是整棵组件。TRNovel 实测主导模式为
`use_future(外层){ tokio::spawn(内层){ state.set(..) } }`——内层 spawn 要求 `State: Send`，故**必须保留**。
保留后：跨线程 `state.set()` 经 `SyncStorage` 线程安全写入 + `Waker`(Send+Sync) 唤醒渲染循环,行为不变。
`tokio::spawn` 只约束其**实参** future 为 Send,不约束外层 `use_future` future,故 use_future 去 Send 与本决策不冲突——TRNovel 在自己的 custom hook 里按需自加 `F: Send`/`T: Send+Sync`,不依赖框架强制。

## Risks / Trade-offs

- **[面广]** → 逐文件改、每步 `cargo build --all-features` + 末尾四件套；用 `grep "unsafe impl Send"` 收敛到 0 作完成判据。
- **[保留多线程 runtime]** → examples 的 `#[tokio::main]`(默认多线程)**不改**：渲染循环 future 即便 `!Send` 也经 `block_on` 正常驱动（block_on 不要求 Send），同时多线程 runtime 让用户的 `tokio::spawn`/`spawn_blocking` 真并行。**不要**退 `current_thread`——`tokio::spawn` 任何 flavor 都要求 Send，退了救不了 Send 错还丢了 spawn 并行。预期无 example 需改;若真报 Send 错,是别处的真问题,排查而非退 flavor。
- **[State 仍 Send+Sync 而组件树不再]看似不一致,实为有意]** → `use_future` 是 UI 线程内联 poll 的协作式异步(只放会 await 的活);CPU/阻塞重活须经 `tokio::spawn`/`spawn_blocking` 移出 UI 线程并回写 State——故 `State` 保留 Send（见决策 6）。这不是不一致,而是「树不跨线程、State 句柄跨线程」的精准划分。

## Migration Plan

1. `use_future`：LocalBoxFuture + 去 Send。
2. `Hook`/`Component`/`AnyComponent`：去 Send+Sync bound。
3. `Props`：安全 trait + 改 derive；删 `AnyProps` unsafe。
4. 删 `SendBlock` + 改 tree_select/scroll_view 边框字段。
5. `TextParagraph`/adapter/router/context：删 unsafe（adapter Props 改安全 impl）。
6. `cargo build --all-features` 收敛报错；examples 编译；末尾四件套 + 现有测试全绿；`grep unsafe impl Send` = 0。

回滚：逐步提交，出问题还原对应提交。

## Open Questions

- 个别 example 是否因用户 async 写法需 `current_thread`——编译时见分晓。
