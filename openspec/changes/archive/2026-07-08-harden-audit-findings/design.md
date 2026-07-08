## Context

两份独立审查(72-agent 工作流综合裁定 + codex 报告)交叉印证了一批缺陷。本变更跨 element/props/render/hooks/atom/router/components/macros 多模块,且含若干 BREAKING,需先固定技术决策再实施。一手代码核对得出三条关键事实,直接决定方案形态:

1. **`generational-box` 0.7.9 的 `SyncStorage` 借用是非阻塞的**(底层 parking_lot 的 `try_read/try_write` 在锁冲突时立即返回 `Err(AlreadyBorrowed*)`,不阻塞)。故 `use_state.rs:206-221`/`atom/mod.rs:172-187` 的 `loop { … AlreadyBorrowedMut => try_write … continue }` 是**自旋忙等**(占满 CPU 卡住渲染线程),而非 OS 阻塞。删掉自旋后,重入会经 `try_*` 立即返回 `None`、`read()/write()` 的 `expect` 快速 panic——**无需改 `UnsyncStorage` 即可根治**。
2. **`ComponentHelperExt::copy` 并非死代码**(`any_element.rs:49/60/86` 在调用)。工作流的该 elegance 项是误报,**不删**。
3. **首跑漏跑只存在于 `use_async_effect`**(`use_effect.rs:62` 的 `if hook.deps_hash != dep_hash` 无 `is_none()` 兜底);`use_memo`(`use_memo.rs:50`)有 `|| memoized_value.is_none()` 兜底,首跑正常。

约束:State/Atom 须保留 `SyncStorage` + `Send + Sync`(用户在后台 `tokio::spawn` 写状态是有意能力,延续归档 `drop-send-sync` 决策 6);仓库约定无单元测试,回归以四件套(`--all-features`)+ 针对性 trybuild/离屏 harness 为准。

## Goals / Non-Goals

**Goals:**
- 消除公开安全 API 的潜在悬垂指针(`AnyProps::borrow`/`AnyElement::from`)。
- 消除用户输入直接可达的 panic(Outlet 无兜底、ScrollView u16 溢出、动态路由跨段、TreeSelect 重复 id)。
- 根治 `try_read` 自旋忙等(保留 SyncStorage 前提下)。
- 修正 ScrollView 显隐错配/受控失效、hooks 依赖碰撞与首跑、异步收尾帧。
- 收敛重复样板(ReactiveHandle 单一真源、ElementExt 拆分)与确证的死代码/笔误。

**Non-Goals:**
- **不**把 State/Atom 改 `UnsyncStorage`、**不**移除 `Send + Sync`(用户决策:支持后台异步写入)。
- **不**引入脏标记/局部跳过 update 的细粒度重渲(单独演进,见 proposal 外)。
- **不**重写整体渲染/协调模型、不改终端泛型。
- **不**删除 `ComponentHelperExt::copy`(经核实有调用方)。

## Decisions

### 决策 1:借用句柄生命周期收紧到借用期

- `AnyProps::borrow(&mut self) -> Self` 改为 `fn borrow(&mut self) -> AnyProps<'_>`,返回生命周期绑定到 `&mut self` 借用期(`AnyProps<'a>` 的 `borrow` 产出受 `&'_ mut self` 约束的更短句柄)。
- `impl From<&'a mut AnyElement<'b>> for AnyElement<'b>` 改为 `for AnyElement<'a>`(返回借用期 `'a`,不放大到 `'b`)。
- **理由**:当前 `borrow` 返回与源同寿(`'a`/`'b`)的句柄但只持裸指针,借用检查器无法阻止它逃逸出实际可变借用作用域 → 公开安全路径(`Route::borrow`/`Outlet`)潜在悬垂。收紧后由借用检查器强制"借用副本不超过 `&mut` 借用期"。
- **连带**:`outlet.rs:43` `AnyElement::from(&mut current_route.component)` 与 `element_ext.rs` 的 `props_mut(&mut self) -> AnyProps<'_>` 已是"同步立即消费",收紧后仍编译通过(借用未逃逸)。
- **备选**:仅加文档/`# Safety` 注释——否决,留隐患;运行时检查——无必要,编译期可表达。
- 顺带:`AnyProps` 加 `type_id: TypeId` 字段,`downcast_*_unchecked` 加 `debug_assert_eq`(release 零开销)。

### 决策 2:`try_*` 删自旋循环,重入快速 panic(保留 SyncStorage)

- `State::try_read`/`AtomState::try_read` 改为不重试:`self.inner.try_read().ok().map(|inner| …)`——任何 `Err`(含 `AlreadyBorrowedMut`/`Dropped`)都返回 `None`。删除 `loop`/`AlreadyBorrowedMut => try_write => continue` 分支。
- `read()/write()` 保持 `try_*().expect(...)`:持守卫重入时立即 panic(可定位),不再忙等。
- **理由**:见 Context 事实 1——parking_lot 非阻塞,删自旋即得快速失败语义,且**无需更换存储后端**,与"保留 SyncStorage 支持后台写"零冲突。
- **备选**:换 `UnsyncStorage`——已被用户否决(丢失后台跨线程写能力)。

### 决策 3:`ReactiveHandle<T, N>` 泛型单一真源

- 抽出内含 `GenerationalBox<ReactiveValue<T, N>, SyncStorage>` 的泛型核心,`N: Notifier` 为变更通知策略:`SingleWaker`(State)与 `WakerMap`(AtomState,按组件 key 的 `HashMap<_, Waker>`)。
- read/`try_*`/write/`Display`/`Debug`/`Hash`/`PartialEq`/`PartialOrd`/`Eq` 只写一份;`reactive_ops` 宏并入(或对 `ReactiveHandle` 单次展开)。`State`/`AtomState` 退化为 `ReactiveHandle<T, SingleWaker>`/`<T, WakerMap>` 的薄 newtype(保留公开类型名与现有方法签名,避免 API 破坏)。
- **理由**:`use_state.rs:117-320` 与 `atom/mod.rs:107-275` 当前逐字符同构两份,改一处漏一处(自旋忙等就两处都有)。保留 newtype 而非直接别名,便于各自挂 `use_state`/`use_atom` 专属构造与文档。
- **备选**:仅抽 try_/read/write 留比较/格式化重复——收益不全;直接 `type State<T> = ReactiveHandle<...>` 别名——丢失为各自加内置方法/文档的空间。

### 决策 4:依赖比较 `Hash` → `PartialEq + Clone`,存 `Option<D>`(**BREAKING**)

- `use_memo`/`use_effect`/`use_async_effect` 的 `D: Hash` 改为 `D: PartialEq + Clone`;hook 内存 `Option<D>`(`None` = 未跑),比较 `last.as_ref() != Some(&deps)` 决定是否重算/重跑。
- 一举解决两问题:消除 `u64` 哈希碰撞漏更新;`Option<D>` 的 `None` 天然保证**首次必跑**(同时修掉 `use_async_effect` 的 `deps_hash=0` 漏跑)。
- **理由**:对齐 React useMemo/useEffect 的"依赖浅相等"语义,比哈希更正确。多数 deps 是 `Copy` 元组/基本类型,满足 `PartialEq + Clone`。
- **权衡/BREAKING**:依赖 bound 由 `Hash` 变为 `PartialEq + Clone`,极少数仅实现 `Hash` 的 deps 需调整;存一份 `D` 副本(小值,可忽略)。
- **备选(保守)**:仅把 `deps_hash` 改 `Option<u64>` 修首跑、碰撞留文档——更小但不根治碰撞;用户选"全部纳入/彻底",故取 PartialEq 方案。

### 决策 5:Outlet 优雅兜底,动态路由尾段锚定

- Outlet 无匹配且无根路由时:渲染空(`Fragment`),不 `expect`。可选在 `RouterProviderProps` 增 `not_found: Option<Element>` 兜底(本轮先做"渲染空 + debug 告警",NotFound prop 视需要)。
- 动态路由正则:尾部追加段边界锚 `(?:/|$)`(或匹配后校验 `rest` 为空或以 `/` 起始),与静态分支语义对齐;补 `/users/:id/edit` 不匹配 `/users/42/edit-more` 的回归测试。

### 决策 6:ScrollView 复用 `ratatui::Layout` 推导尺寸

- 用 `Layout::split`/`Layout::spacers` 得子区域与内容总尺寸,替代手算 `constraint_sum`(裸 u16 乘法 + 把 Min/Max/Fill 当固定长度),根除溢出且弹性约束语义正确。
- 负 `gap` 累加 `.max(0)` + `saturating_add` 收口。
- "是否预留滚动条/缩小内容区"抽成 calc 与 render 共用的单一函数(同一内容尺寸来源、同一 ±1 约定),消除两套公式漂移。
- 横纵镜像复制用"按 `Direction` 取主/交叉轴"小工具收敛。
- `UseScrollImpl`:`use_hook` 取回可变引用后每帧回写 `scroll_view_state`/`has_block`(与同文件 scrollbars 的 use_effect 同步方式一致)。

### 决策 7:`use_route_mut` 移除(**BREAKING**)

返回 `Context::owned` 临时克隆的可变引用、改动每帧被丢弃,是误导性 API。直接删除;迁移指引见 spec REMOVED 语义(用 use_state/AtomState 或 route state)。

### 决策 8:组件树契约 + 确证的清理

- 透明布局:`instantiated_component.rs` update 末尾,透明且 `children` 为空时 `self.layout_style = LayoutStyle::default()`。
- `draw` 配对处加 `debug_assert_eq!(children_areas.len(), children.len())`,trait 文档补契约。
- `poll_change` 三路:加注释/抽 `poll_all` 固定"全 poll 不短路";`Hook`/`Component` 全员 `Unpin`,`poll_change` 去掉多余 `Pin` 投影(签名 `&mut self`)——**纯内部 trait 重构,行为不变**。
- `ElementExt` 拆为 crate 内 `ElementRepr`(`key/props_mut/helper`,Sealed)与面向用户的 `App`/`Runnable`(`render_loop/fullscreen` 一份默认实现),消除 `AnyElement`/`&mut AnyElement` 四份重复样板。
- 确证清理:`DropRowImpl`→`DropRawImpl` 笔误(或整体改函数指针免 Box);`render_loop` 中被丢弃的 `terminal.events()`(**实现时先核实**确为无效订阅再删);`TextParagraph` by-value `Widget`(**实现时先 grep 调用方**,仅当确认仅 `&TextParagraph` 路径在用才删)。**不删** `ComponentHelperExt::copy`(有调用方)。

### 决策 9:按 capability 分批实施 + 每批验证

P0(安全/崩溃)→ P1(正确性)→ P2(契约/语义)→ P3(重构),每批末尾跑四件套,降低大面积改动的回归风险(见 Migration Plan)。

## Risks / Trade-offs

- **[生命周期收紧波及 Outlet/Route::borrow,可能触发借用错误]** → 这正是目的(暴露真实逃逸);若 Outlet 现有用法因收紧编译失败,说明存在真实悬垂风险,按"立即消费/不逃逸"调整调用点,而非放宽签名。
- **[ReactiveHandle 抽象面广]** → 保留 `State`/`AtomState` 公开类型名与方法签名,改动收敛在内部;以现有 25 lib 单测 + 四件套验证行为等价。
- **[依赖比较 BREAKING]** → 在 CHANGELOG/迁移说明标注 `Hash`→`PartialEq + Clone`;examples 先行验证(deps 多为 Copy 值)。
- **[ScrollView 改用 Layout::split 可能微调像素分配]** → 补离屏 harness 冒烟(溢出、显隐临界、受控切换),对比改造前后渲染快照。
- **[死代码误删]** → tree.rs `events()`、TextParagraph by-value 标"先核实调用方",`ComponentHelperExt::copy` 明确不删(已知误报)。
- **[面广]** → 分批 + 每批四件套;`grep` 收敛(如确认 `try_read` 无 `loop` 残留)。

## Migration Plan

1. **P0a 借用安全**:`props.rs`(borrow 签名 + TypeId + DropRaw)、`any_element.rs`(From 返回 'a)、`context.rs`(二次借用诊断);补 trybuild。
2. **P0b 反 panic**:`outlet.rs`(兜底)、`router/mod.rs`(动态尾段锚定)、`scroll_view/mod.rs`(u16 溢出 + 负 gap)、`tree_select.rs`(缓存 Tree::new)。
3. **P1 状态/hooks**:`use_state.rs`/`atom/mod.rs`(删自旋 + ReactiveHandle 抽象)、`use_memo.rs`/`use_effect.rs`(PartialEq + Option<D> + 首跑)、`use_future.rs`/`use_effect.rs`(收尾帧)、`tree.rs`(draw `?` + restore guard)。
4. **P1 ScrollView 一致性**:显隐单一函数 + UseScrollImpl 逐帧同步 + 复用 Layout 去镜像复制。
5. **P2 契约/语义**:透明布局空子树重置、calc debug_assert、poll_change 注释、路由优先级文档、移除 use_route_mut。
6. **P3 重构**:Pin 简化、ElementExt 拆分、宏 codegen 中转清理、with_layout_style 友好报错、确证死代码/笔误清理。
7. **验证**:每批后四件套(`--all-features`)+ 现有单测;末尾补本变更新增的 trybuild/段边界/ScrollView harness 测试全绿。

回滚:逐批提交,出问题还原对应批次提交。

## Open Questions

- Outlet 兜底是"渲染空"足够,还是本轮就加 `RouterProvider { not_found }` prop?倾向先渲染空 + debug 告警,prop 视需要。
- ScrollView 是否完全弃用手算改全量 `Layout::split`,还是仅在求和处收口?倾向前者(根治),但若 `Layout::split` 语义与现有视觉有差,退回"收口 + 单一显隐函数"的最小修。
- `TextParagraph` by-value `Widget` 与 `render_loop` 中 `terminal.events()` 的删除,以实现时 grep 调用方结论为准。
