# Implementation Tasks

> 框架侧（1–4 组)必须先全绿（`cargo test --locked --all-features --workspace --lib --tests --examples` + 逐 example 手验)再动 TRNovel（5–7 组)。无单元测试,正确性靠 examples + 手验。

## 1. 框架核心运行时

- [x] 1.1 (T1) 新建 `packages/ratatui-kit/src/input/mod.rs`：定义 `EventResult`/`EventPriority`/`EventScope`/`EventOptions`/`InputLayer`/`LayerId`/`CurrentLayer`/`LayerEntry`/`HandlerEntry`/`InputRuntime`。`dispatch` 用 `std::mem::take(&mut self.handlers)` 取出遍历(消除自借用);闭包 `Box<dyn FnMut(Event)->EventResult>` 不要求 `Send+Sync`(单线程);`area` 用 `Rc<Cell<Rect>>`。在 `lib.rs` 挂模块 + prelude 导出公共类型。验证：`cargo build --all-features`。
- [x] 1.2 (T1) 实现 `InputRuntime` 方法：`begin_frame`(清空层/handler + 铸造 root 层)、`root_layer`、`mint_layer_id`(单调自增)、`push_layer(open, blocks_lower)`(open=false 仍 mint id 但不入 layers)、`register_handler(...)`、`dispatch(event)`。**dispatch 分两 phase**:Phase 1 全部 `Global`(按 priority/order,`Consumed` 终止全程);Phase 2 活跃层(从栈顶遇首个 `blocks_lower` 截断)按 **`(层 z-order desc, priority desc, order asc)`** ——**z-order 第一键、不跨层比 priority**、`Consumed` 早停;`hit_test` 鼠标命中过滤;绑定到未登记/inactive 层的 handler 静默跳过。`dispatch` 用 `std::mem::take` 取出 handlers 遍历。
- [x] 1.3 (新增,审核第 5 点) `InputRuntime` 纯逻辑单测 `#[cfg(test)] mod tests`(不启动真实终端,经 `push_layer`/`register_handler`/`dispatch` 直接构造)：覆盖 ①`blocks_lower` 截断背景层 ②嵌套 `blocks_lower` 只激活最顶层 ③`Consumed` 截断后续 handler ④`Ignored` 继续传播 ⑤**层 z-order 优先于 priority**(下层 high 不抢上层 normal)⑥`Global` 独立 phase 的排序与 `Consumed`/`Ignored` 语义 ⑦handler 绑定 missing/inactive 层时不调用 ⑧`hit_test` 区域外跳过。验证：`cargo test --all-features -p ratatui-kit input::`。
- [x] 1.4 (T2) `context.rs`：`SystemContext` 加 `pub input: InputRuntime` 字段,`new()` 初始化 `input: Default::default()`;`should_exit`/`exit` 不变。验证：`cargo build --all-features`。
- [x] 1.5 (T3) `terminal/mod.rs` 瘦身：删 `TerminalEventsInner`/`TerminalEvents`/`impl Stream`/`#[cfg(test)] empty()`/`subscribers` 字段/`received_ctrl_c` 字段/`events()`/`wait()`/`received_ctrl_c()` getter;新增**异步泛型** `pub async fn next_event(&mut self) -> Option<T::Event>`;保留 `TerminalImpl::received_ctrl_c(event)` 关联函数。`UpdaterTerminal` trait 删 `events()` 仅留 `insert_before`。`render/harness.rs` 的 `NoopTerminal` 删 `events()` impl + `TerminalEvents` 引用。验证：全仓 `grep received_ctrl_c()/\.events()/TerminalEvents` 无遗留调用方。
- [x] 1.6 (T4) `render/tree.rs` `render_loop` 改造：`update_once` 第一行调 `self.system_context.input.begin_frame()`(必须在 `ContextStack::root` 之前);`select` 双路 `root_component.wait()` / `terminal.next_event()`——**`Left(())`**(树/状态变更)→ `continue` 仅重渲染;**`Right(Some(event))`** → `auto_quit_on_ctrl_c` 开启且 `CrossTerminal::received_ctrl_c(event.clone())` 命中即 break,否则 `system_context.input.dispatch(event)` 后**无条件 `continue`** 复查 `should_exit`(红队必改 1);**`Right(None)`** → break。删 `render()` 内 `received_ctrl_c()` 检查。验证：`begin_frame` 与 `ContextStack::root` 借用不冲突;`cargo build --all-features`。

## 2. 框架 Hooks 与组件

- [x] 2.1 (前置,审核第 1 点) 确立 context-aware hooks 约定(框架核心前置)：核验 `Hooks::use_context_mut` 在 `context==None` 时 panic(`use_context.rs:40`),`AnyComponent::update`(`component/mod.rs:136`)转发 `context==None` 的 hooks,仅 `#[component]` 宏(`component.rs:185`)做 `with_context_stack`。确立约定——**函数组件**开箱即用;**手写 Component** 用 `use_input_layer`/`use_event_handler` 前 MUST 先 `let mut hooks = hooks.with_context_stack(updater.component_context_stack());` 且与后续 `&mut updater` 操作时序分离;**Modal** 走 `updater.get_context_mut::<SystemContext>()` 不经 hooks。**不**在框架层统一注入(`Component::update` 同持 hooks 与 `&mut updater`,借用冲突),`#[component]` 宏不变。这是 2.2(hook 实现假设 context 已注入)与 2.5(ScrollView 依赖)的前置。验证：约定文档化于 design D3a。
- [x] 2.2 (T5) 新建 `hooks/use_input.rs`：`UseInputLayer::use_input_layer(open, blocks_lower) -> InputLayer` 与 `UseEventHandler::use_event_handler(_with_options)`,Sealed trait。**层与 handler 均在函数体内**经 `self.use_context_mut::<SystemContext>().input` 当帧登记,守卫**块内即弃**(红队必改 2、3);`use_hook` 占顺序槽保证 Hook 顺序稳定;`Current` 归属先 `try_use_context::<CurrentLayer>()` 后回落 `root_layer()`。`UseEventHandlerImpl` 持 `Rc<Cell<Rect>>` 在 `pre_component_draw` 回填 area。`hooks/mod.rs` + `lib.rs` prelude 导出。验证：闭包不要求 `Send+Sync`;`cargo build --all-features`。
- [x] 2.3 (T6) 删 `hooks/use_events.rs` 整文件;`hooks/mod.rs` 删 `mod use_events; pub use use_events::*;`,加 `mod use_input; pub use use_input::*;`。`hooks/use_size.rs` 的 `use_terminal_size` Resize 监听改 `use_event_handler(EventScope::Global, EventPriority::Normal, |e| { if Resize {..} EventResult::Ignored })`(Resize 必须 Global + 返回 Ignored 让多订阅者都收)。验证：全仓框架内无 `use_events`/`use_local_events` 引用;`cargo build --all-features`。
- [x] 2.4 (T7) `components/modal.rs` 改造：`ModalProps` 加 `layer: Option<InputLayer>` 与 `blocks_lower: Option<bool>`(读取时 `unwrap_or(true)`);`update` 体内用 `updater.get_context_mut::<SystemContext>()`(降级 None 版)——外传 `layer` 时复用其 id **不重复 push**,否则自 `push_layer(open, blocks)`;**取得 id 后 drop RefMut**(红队必改 3)再 `update_children(children, Some(Context::owned(CurrentLayer(id))))`;`open=false` 不 push 不注入。验证：`examples/modal.rs` 编译;借用纪律到位。
- [x] 2.5 (T8) `components/scroll_view/mod.rs`(手写 Component,依赖 2.1)：**先 `let mut hooks = hooks.with_context_stack(updater.component_context_stack());`,所有 hooks 操作置于 `update_children` 之前**;`use_local_events` → `use_event_handler_with_options(Current, Normal, EventOptions{hit_test:true}, ...)`,disabled 时 handler 首行 `return Ignored`,滚动键命中返回 `Consumed`。验证：`harness` 的 `scroll_view_tests` 通过;鼠标命中等价旧 `in_component`;手写组件不再因 `context==None` panic。

## 3. 框架 Examples

- [x] 3.1 (T9) 新增 canonical `examples/input_mutex.rs` + 根 `Cargo.toml` 注册：演示——(1) 背景列表 `Current` 层;(2) 输入框进输入态开二级 `blocks_lower` 层截断背景;(3) 弹窗 `Layer(h)` 父级 handler **三件套配对**(`use_input_layer(open,true)` + `use_event_handler(Layer(h))` + `Modal(layer: Some(h))`),注释标注「漏传 `h` 给 Modal → 父级 handler 失聪」footgun(审核第 6 点);(4) 非阻塞上层浮层(`blocks_lower=false`)验证 **z-order 优先于 priority**——下层 high-priority 不抢消费上层 normal(审核第 2 点)。验证：`cargo run --example input_mutex` 各路径行为正确(打字时父列表 j/k/Enter 被屏蔽、Esc 退出后恢复;上层浮层先于下层 high 收键)。
- [x] 3.2 (T10) 迁移框架 examples 的 `use_events`/`use_local_events` → `use_event_handler`(Current/Global)：`modal.rs`/`router.rs`/`input.rs`/`store.rs`/`list.rs`/`custom_list.rs`/`scrollview.rs`。验证：`cargo test --locked --all-features --workspace --lib --tests --examples` 全过 + 逐个 `cargo run --example` 手验(尤其 router 切页旧页 handler 自然消失、store 的 atom 写入唤醒不变)。

## 4. 框架侧合入门槛

- [x] 4.1 四件套全绿：`cargo test --locked --all-features --workspace --lib --tests --examples`、`cargo clippy --all-targets --all-features --workspace -- -D warnings`、`cargo fmt --all --check`、`RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items --all-features --workspace --examples`。
- [x] 4.2 `harness.rs` 现有 5 组件渲染测试 + `router_tests` 经 `update_once(NoopTerminal)` 跑 `begin_frame` 但不 dispatch,断言静态渲染输出不变(验证瘦身无回归)。

## 5. TRNovel 共享组件与全局状态

- [ ] 5.1 (T11) `src/components/search_input.rs`：本地 `editing: State<bool>` 替代全局 `is_inputting`;**常开** `use_event_handler(Current, High, ...)` 接 `s`(非输入态)进入 + 输入态分流(Esc 退出/Enter 提交后 `editing.set(false)`/字符输入,命中返回 Consumed);`use_input_layer(editing.get() && is_editing, blocks_lower=true)`。删 `pending_exit` State/延迟 `use_effect`/Enter 分支 set。
- [ ] 5.2 (T11) `src/components/shortcut_info_modal.rs`：改签名加 `on_close: Handler<()>`(或 `open: State<bool>`)句柄 + 内部 `use_event_handler(Current, High, |e| {if i/Esc {on_close(()); Consumed} else Ignored})`(关闭键下沉,红队新发现)。7 处调用方全部接线 `on_close`。
- [ ] 5.3 (T11) `src/components/modal/{confirm,warning,browser_prompt}.rs`：改自开层弹窗——`use_input_layer(open, blocks_lower=true)` 取 `h` + `use_event_handler(Layer(h), High, ...)` + 把 `h` 传给 `Modal(layer: Some(h))`(Modal 不再自开层)。删 `if !open return`;`warning.rs` `open=false` 不注册层(修复「不读 open 仍吃 Esc/q」隐患);`browser_prompt.rs` `open = state.is_some()`。验证：`cargo build`(TRNovel)。
- [ ] 5.4 (T12) `src/app/mod.rs` + `src/app/layout.rs`：删 `is_inputting` 全局 `State<bool>` 整链路(声明 + `ContextProvider` 注入 + 11 处读取 + 7 处写入);各页 `!modal_open`/`current.is_none()` 手算门控删除(由 `blocks_lower` 截断替代)。验证：全仓 `grep is_inputting` 仅剩本地化命名(无全局 context);`ContextProvider` 嵌套减一层;`cargo build`。

## 6. TRNovel 列表导航与简单页面

- [ ] 6.1 (T13) `src/components/{list_select,multi_list_select,select}.rs` + TRNovel 文件树选择用法：`use_events` → `use_event_handler(Current, Normal, ...)`,`is_editing` 守卫保留为 handler **首行 early-return**(不放进 `if`,保证 Hook 顺序稳定),命中返回 `Consumed`。框架侧统一沉淀为通用 `TreeSelect`，文件语义留给 TRNovel。
- [ ] 6.2 (T13) `src/app/layout.rs`：全局 `q`/`g`/`b` 用 `use_event_handler(Current, Normal, ...)`(**非 Global**——须被弹窗/输入态屏蔽),命中返回 `Consumed`。验证：弹窗/输入态打开时 q/g/b 不触发;`cargo build`。
- [ ] 6.3 (T14) 简单页面迁移：`home`/`local_novel`/`select_history`/`book_detail`/`book_source_manager`(+`select_book_source`)/`select_books`(+`find_book`)。`use_events` → `use_event_handler(Current)`;删 `!modal_open`/`!is_inputting` 门控;`select_books` 的 explore Modal 靠 Modal 自动开层。验证：`cargo build`。

## 7. TRNovel 复杂场景、验证与知识库

- [ ] 7.1 (T15) `theme_setting/mod.rs` + `select_color.rs`：双层嵌套——`SelectColor` 的 Modal 自动开层 L1,`Select` 用 `Current` 归 L1;`SearchInput` 进输入态 `use_input_layer` 开 L2(`blocks_lower=true`)截断 L1。
- [ ] 7.2 (T15) `book_source_login.rs`：整页输入层(进页面即开 `blocks_lower` 层)。
- [ ] 7.3 (T15) `read_novel/{mod,read_content,select_chapter}.rs`：三层弹窗(ReadContent/TTS/Info)由 z 序(element 书写顺序)+ `blocks_lower` 处理;`i`/`t` 关闭键**下沉**进各 Modal 内部 handler(页面 handler 仅负责无弹窗时「打开」);`read_content` 保留 `is_scroll` 首行守卫。
- [ ] 7.4 (T15) TTS 6 项 `tts/{mod,settings,download,voice_select}.rs`：全部 `use_event_handler(Current, Normal, ...)` 归 TTSManager 注入层;**保留 `index` 守卫**(priority 无法表达选中第几项)作首行,命中项返回 `Consumed`、未命中返回 `Ignored`(否则吃掉 TTSManager 的 j/k 切 index)。
- [ ] 7.5 (T15) TRNovel 全流程手验：`cargo run`——三层弹窗 z 序正确、select_color 双层输入互斥、弹窗开时背景静默且弹窗本身可操作、关闭恢复、search/Enter 不再误触父列表、Resize/Ctrl+C 正常、退出正常。
- [x] 7.6 (T16) 知识库更新 `dev-notes/knowledge/`：`runtime-architecture.md`(dispatch 时机/begin_frame/事件分发与重绘解耦)、`hooks-and-state.md`(`use_input_layer`/`use_event_handler` 约定、闭包禁触 SystemContext、借用即弃纪律、InputLayer 不可跨帧)、`macros-and-props.md`(Modal 手写组件经 updater 拿 context);记录 z 序依赖书写顺序、`is_inputting`/`pending_exit` 已删根因。
