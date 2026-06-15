## Why

两份独立架构审查(72-agent 工作流综合裁定 + codex 报告)交叉印证了一批真实缺陷:**公开安全 API 的潜在悬垂指针**、**用户输入直接可达的 panic(会拖垮整个 TUI 并破坏终端)**、**状态句柄重入借用忙等/死锁**、**ScrollView 整数溢出与显隐错配**、**hooks 依赖追踪的碰撞/漏跑**,以及若干设计契约缺失与重复样板。框架底子健全(iocraft 风格、unsafe 高度收敛、DX 出色),这些是"边界没收紧"而非"架构错了",集中修复一轮即可显著提升稳健性与整洁度。

## What Changes

### 内存/借用安全(High)
- **收紧 `AnyProps::borrow` 与 `AnyElement::from(&mut …)` 的生命周期**:`AnyProps::borrow` 改为 `fn borrow(&mut self) -> AnyProps<'_>`,把派生句柄绑定到 `&mut self` 借用期;`AnyElement::from(&mut AnyElement<'b>)` 返回借用期 `'a` 而非放大到 `'b`,消除"借用伪装成更长生命周期"的悬垂风险(影响 `Route::borrow`/`Outlet`)。
- **`AnyProps` 增 `TypeId` + `downcast_*` debug_assert**:把"协调阶段已校验 TypeId"这一跨文件隐式不变量变为 debug 期可观测断言(release 零开销)。
- **`context.rs` 同类型二次借用区分报错**:`get_context(_mut)` 区分"未找到"与"已被借用",消除误导性的 "context not found" panic。

### 消除用户输入可达的 panic(High/Medium)
- **`Outlet` 无匹配路由时优雅兜底**:不再 `expect` panic,渲染空内容/可配置 `NotFound`,而非崩溃。
- **动态路由尾部静态段补段边界锚定**:`/users/:id/edit` 不再误匹配 `/users/42/edit-more`(与已有的静态段边界语义对齐)。
- **`ScrollView::constraint_sum` 整数溢出收口**:`Fill(i)`/大 `Percentage`/`Ratio` 改为高位计算 + 饱和收口(根因上改为复用 `ratatui::Layout` 推导尺寸),消除 debug 下 panic、release 下错乱。
- **`TreeSelect::draw` 不再 `unwrap`**:重复 identifier 时在构造期校验/缓存 `Tree::new` 结果,draw 只消费缓存。

### 渲染错误恢复(Medium)
- **`render` 循环 `terminal.draw` 错误经 `?` 传播**,并以 guard/Drop 确保失败路径也恢复终端(与 ratatui panic hook 形成双保险)。

### ScrollView 一致性(Medium/Low)
- **滚动条显隐统一为单一公式/单一内容尺寸来源**,消除 calc 与 render 两套漂移导致的临界尺寸错配。
- **`UseScrollImpl` 逐帧同步 props**(`scroll_view_state`/`has_block`),修复运行中切换受控/block 失效(对齐 CLAUDE.md 第 6 节参数化 hook 约定)。
- **负 `gap` 收口**:避免 `as u16` 回绕导致的溢出 panic/超大分配。

### Hooks 依赖追踪(Medium/Low)
- **`use_memo`/`use_async_effect` 依赖比较改为可靠相等**(消除仅存 hash 的碰撞漏更新),并保证**首次必跑**(修 `deps_hash = 0` 与首帧依赖哈希恰为 0 时漏跑首次 effect)。
- **`use_future`/`use_async_effect` 完成时触发收尾帧**:`poll_change` 在 future 刚完成的就绪分支返回 `Ready` 一次,避免只读副作用用法停在倒数第二帧。

### 响应式状态核心(治标 + 去重,**不改存储后端**)
- **`State::try_read`/`AtomState::try_read` 不再忙等**:`try_*` 直接返回 `None`,删除针对 `AlreadyBorrowedMut` 的死循环重试;`read()/write()` 负责快速失败 panic。文档明确严禁持读守卫期间再写同一 state。
- **`State`/`AtomState` 抽成单一泛型 `ReactiveHandle`**:读写访问层 + `Display/Debug/Hash/PartialEq/PartialOrd/Eq` 合并为单一真源(差异仅"单 Waker vs 多 Waker 通知策略"),与 `reactive_ops` 宏合并;`State`/`AtomState` 退化为薄 newtype/别名。
- **保留 `SyncStorage` + `Send + Sync`**(局部 State 与全局 Atom 皆然):支持后台 `tokio::spawn` 写状态是有意能力(延续 `drop-send-sync` 决策 6),本提案**不**改 `UnsyncStorage`、**不**移除 `Send + Sync`。

### 组件树契约(Low,行为契约化)
- **透明布局空子树重置**:`#[component]` 透明组件该帧无子节点时把 `layout_style` 重置为 `default()`,消除"保留上一帧旧值"的布局粘连。
- **`calc_children_areas` 区域数=子节点数契约**:`draw` 的 `zip` 处加 `debug_assert`,把隐式契约显式化。
- **`poll_change` 三路全 poll 不变量**:加注释/辅助函数固定"三路都求值以注册 Waker",防后续重构改成短路求值丢唤醒;并去掉全员 `Unpin` 下多余的 `Pin` 投影仪式。

### 设计契约澄清 / 优雅性清理(全部纳入)
- **路由声明顺序/特异性语义**:文档明确"声明顺序即优先级"(或引入静态优先特异性);**移除误导性的 `use_route_mut`**(返回的是临时克隆、改动被丢弃)。**BREAKING**(公开 hook 移除)。
- **`#[with_layout_style]` 用于非具名字段结构体给出友好 `compile_error!`**,替代误导性的下游展开报错。
- **死代码 / 样板清理**:删除 `ComponentHelperExt::copy`、`render_loop` 中被丢弃的 `terminal.events()`、`TextParagraph` 的 by-value `Widget` 实现;`props.rs` 的 `DropRaw`/`DropRowImpl`(笔误)简化为函数指针;`ElementExt` 拆分为内部 `ElementRepr`(声明层)与面向用户的应用入口(`render_loop/fullscreen`),消除四份重复样板;宏 codegen(`adapter.rs`/`element.rs` 无 props/无 children 分支)去掉多余中转绑定。

## Capabilities

### New Capabilities
- `element-borrow-lifetimes`: `AnyProps`/`AnyElement` 借用句柄的生命周期与 TypeId 安全契约——借用派生绑定到 `&mut self` 借用期、downcast 前 TypeId 已校验且 debug 期可断言、context 同类型二次借用快速可诊断。
- `reactive-state-core`: 响应式状态句柄(`State`/`AtomState`)的统一核心——单一泛型 `ReactiveHandle` 真源、`try_*` 非阻塞返回 `None` 不忙等、保留 `SyncStorage`/`Send+Sync` 以支持后台写入、持守卫重入约定。
- `scroll-view-layout`: ScrollView 子区域/内容尺寸推导与滚动条显隐契约——尺寸计算无整数溢出且弹性约束语义正确、显隐用单一公式与单一尺寸来源、滚动 hook 逐帧同步 props。
- `render-error-recovery`: 渲染循环错误处理契约——`terminal.draw` IO 错误经 `Result` 传播、失败路径终端恢复有保障、draw 阶段对可恢复的数据错误(如重复 identifier)不 panic。
- `hook-dependency-tracking`: 依赖型 hook(`use_memo`/`use_async_effect`)的依赖比较与触发契约——依赖相等判断可靠(无 hash 碰撞)、首次必跑、异步完成触发收尾重渲。
- `component-tree-contracts`: 组件树运行时内部契约——透明布局空子树重置、`calc_children_areas` 区域数=子节点数、`poll_change` 三路全 poll 不丢唤醒。

### Modified Capabilities
- `router-path-matching`: 在既有"静态段边界匹配/动态参数提取/正则单次编译"基础上,补"动态路由尾部静态段同样按段边界锚定"、"`Outlet` 无匹配优雅兜底而非 panic"、"路由优先级/`use_route_mut` 语义"三项。
- `element-head-codegen`: 在既有 `ParsedElementHead` 单一真源基础上,补"`#[with_layout_style]` 误用于非具名结构体时给出友好 `compile_error!`"与宏 codegen 中转清理(行为不变)。

## Impact

- **公开 API(BREAKING)**:移除 `use_route_mut`;`Outlet` 无匹配行为由 panic 改为兜底渲染。`AnyProps::borrow` 签名收紧(内部 API,经 `routes!`/协调路径,用户通常不直接调用)。
- **代码(主库)**:`element/{any_element,element_ext}.rs`、`props.rs`、`context.rs`、`hooks/{use_state,use_memo,use_effect,use_future,use_router}.rs`、`atom/{mod,use_atom}.rs`、`reactive_ops.rs`、`render/tree.rs`、`component/{mod,instantiated_component,component_helper}.rs`、`components/{scroll_view/*,tree_select,text}.rs`、`components/router/{mod,outlet}.rs`。
- **代码(宏库)**:`macros/src/{with_layout_style,adapter,element}.rs`。
- **不改动**:状态存储后端(`SyncStorage` 保留)、`Send + Sync`(保留,支持后台 spawn)、终端泛型、整体渲染/协调模型。
- **测试**:补 trybuild(`with_layout_style` 误用、生命周期收紧)、动态路由段边界回归测试、ScrollView 离屏 harness 冒烟(溢出/显隐临界);其余以四件套(`--all-features`)+ 现有单测不回归为准。
- **风险**:中——面广但多为局部、低耦合;按 capability 分批改、每批四件套验证,确保运行时行为(除显式 BREAKING 两项)不变。
