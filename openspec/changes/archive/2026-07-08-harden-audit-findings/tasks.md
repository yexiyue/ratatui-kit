> 实施前先调用 `/dev-workflow` skill 加载知识库;每批改动后跑四件套(`--all-features`)。

## 1. 借用安全(P0a)

- [x] 1.1 `props.rs`:`AnyProps::borrow(&mut self) -> Self` 改为 `fn borrow(&mut self) -> AnyProps<'_>`(返回绑定 `&mut self` 借用期的句柄)
- [x] 1.2 `props.rs`:`AnyProps` 增 `type_id: TypeId` 字段(`owned`/`borrowed`/`borrow` 构造时记 `TypeId::of::<T>()` / 透传),`downcast_ref_unchecked`/`downcast_mut_unchecked` 加 `debug_assert_eq!` 校验
- [x] 1.3 `props.rs`:修正 `DropRowImpl` → `DropRawImpl` 笔误(或将 `DropRaw`/`DropRowImpl` 整体改为无捕获函数指针、免 Box 分配)
- [x] 1.4 `element/any_element.rs`:`impl From<&'a mut AnyElement<'b>> for AnyElement<'b>` 改为 `for AnyElement<'a>`(借用期不放大);修复因此波及的 `outlet.rs:43` 等调用点(确保借用立即消费、不逃逸)
- [x] 1.5 `context.rs`:`get_context`/`get_context_mut` 区分"未找到"与"已被借用"——downcast 命中但 `try_borrow(_mut)` 失败时报"context 已被借用"
- [x] 1.6 补 trybuild:派生借用 `AnyElement`/`AnyProps` 逃逸出源 `&mut` 作用域的用例必须编译失败

## 2. 消除用户输入可达的 panic(P0b)

- [x] 2.1 `components/router/outlet.rs`:无匹配且无根路由兜底时渲染空(`Fragment`)+ debug 告警,删除 `expect("No matching route found")`
- [x] 2.2 `components/router/mod.rs`:动态路由正则尾部追加段边界锚 `(?:/|$)`(或匹配成功后校验剩余串为空或以 `/` 起始),与静态分支语义对齐
- [x] 2.3 `components/router/mod.rs`:补回归测试——`/users/:id/edit` 不匹配 `/users/42/edit-more`,匹配 `/users/42/edit` 与 `/users/42/edit/sub`
- [x] 2.4 `components/scroll_view/mod.rs`:`constraint_sum` 的 `Fill`/`Percentage`/`Ratio` 计算提升位宽 + 饱和收口(优先改为复用 `ratatui::Layout::split` 推导尺寸),消除 `u16` 溢出
- [x] 2.5 `components/scroll_view/mod.rs`:负 `gap` 累加 `.max(0)` + `saturating_add` 收口;`sum_count == 0` 提前短路
- [x] 2.6 `components/tree_select.rs`:`new`/`update` 阶段构造并校验/缓存 `Tree::new` 结果,`draw` 仅消费缓存或渲染占位(去掉 `.unwrap()`)

## 3. 响应式状态:去自旋忙等 + 单一真源(P1)

- [x] 3.1 `hooks/use_state.rs`:`try_read` 删除 `loop`/`AlreadyBorrowedMut => try_write => continue`,改为任何 `Err` 即 `None`;确认持守卫重入经 `read()/write()` 的 `expect` 快速 panic(不忙等)
- [x] 3.2 `atom/mod.rs`:`try_read` 同样删除自旋循环
- [x] 3.3 抽出泛型 `ReactiveHandle<T, N>`(`N: Notifier`,实现 `SingleWaker`/`WakerMap`),把 read/`try_*`/write/`Display`/`Debug`/`Hash`/`PartialEq`/`PartialOrd`/`Eq` 收敛为单一真源;`reactive_ops` 宏并入
- [x] 3.4 `State`/`AtomState` 退化为 `ReactiveHandle` 薄 newtype,**保留公开类型名与现有方法签名**;保留 `SyncStorage` + `Send + Sync`(不改 UnsyncStorage、不去 Send+Sync)
- [x] 3.5 `grep -rn "loop" hooks/use_state.rs atom/mod.rs` 确认无 `try_read` 自旋残留;现有 25 lib 单测通过

## 4. Hooks 依赖追踪与异步收尾(P1)

- [x] 4.1 `hooks/use_memo.rs`:`D: Hash` → `D: PartialEq + Clone`,`UseMemoImpl` 存 `Option<D>`,值比较决定重算;首次(`None`)必跑
- [x] 4.2 `hooks/use_effect.rs`:`use_effect`/`use_async_effect` 同步改 `PartialEq + Clone` + `Option<D>`,修复 `use_async_effect` 首帧 `deps_hash=0` 漏跑
- [x] 4.3 `hooks/use_future.rs` + `hooks/use_effect.rs`:`poll_change` 在 future 刚就绪那次返回 `Poll::Ready(())` 触发收尾帧(仍 pending 的不变)
- [x] 4.4 调整受 `PartialEq + Clone` bound 影响的 examples/内部 deps 用法(若有)

## 5. 渲染错误恢复(P1)

- [x] 5.1 `render/tree.rs`:`terminal.draw(...)` 改 `?` 传播,删除 `expect`
- [x] 5.2 `render/tree.rs`:用 guard/Drop 确保正常返回与 `Err` 返回路径都恢复终端(panic 路径仍由 ratatui panic hook 兜底)

## 6. ScrollView 一致性(P1)

- [x] 6.1 `components/scroll_view/`:把"是否预留滚动条并缩小内容区"抽成 `calc_children_areas` 与 `render_scrollbars` 共用的单一函数(同一内容尺寸来源、同一 ±1 约定),消除两套漂移公式
- [x] 6.2 `components/scroll_view/mod.rs`:`UseScrollImpl` 取回 `use_hook` 可变引用后每帧回写 `scroll_view_state`/`has_block`
- [x] 6.3 `components/scroll_view/mod.rs`:横纵镜像复制用"按 `Direction` 取主/交叉轴"工具收敛 140 行重复
- [x] 6.4 补 ScrollView 离屏 harness 冒烟测试(大 Fill 溢出、显隐临界尺寸、运行中受控/block 切换)

## 7. 组件树契约与路由语义(P2)

- [x] 7.1 `component/instantiated_component.rs`:透明布局且本帧无子节点时,`layout_style` 重置为 `LayoutStyle::default()`
- [x] 7.2 `component/mod.rs` + `instantiated_component.rs`:`draw` 配对处 `debug_assert_eq!(children_areas.len(), children 数)`;trait 文档补"区域数=子节点数"契约
- [x] 7.3 `component/instantiated_component.rs`:`poll_change` 三路加注释/抽 `poll_all` 辅助,固定"三路全 poll、禁短路"
- [x] 7.4 `hooks/use_router.rs`:移除 `use_route_mut`(**BREAKING**),同步清理导出与文档
- [x] 7.5 文档:在路由相关 doc 注释/README 明确"声明顺序即优先级"(静态路由置于同前缀动态路由之前)

## 8. 优雅性重构与宏健壮性(P3)

- [x] 8.1 `Hook`/`Component` 的 `poll_change` 去掉多余 `Pin` 投影(全员 `Unpin`,签名改 `&mut self`)——纯内部 trait 重构,行为不变
- [x] 8.2 `element/element_ext.rs`:拆为 crate 内 `ElementRepr`(`key/props_mut/helper`,Sealed)与面向用户的 `App`/`Runnable`(`render_loop`/`fullscreen` 一份默认实现),消除 `AnyElement`/`&mut AnyElement` 四份重复样板
- [x] 8.3 `macros/src/with_layout_style.rs`:对元组/单元结构体产生指向该结构体的友好 `compile_error!`("只能用于具名字段结构体");补 trybuild
- [x] 8.4 `macros/src/adapter.rs` + `element.rs`:无 props/无 children 分支去掉多余 `let mut` 中转,adapter 输出对齐 `to_element_expr` 的外层括号约定
- [x] 8.5 确证清理:grep 核实后删除 `render_loop` 中被丢弃的 `terminal.events()`(确为无效订阅)、`TextParagraph` by-value `Widget`(确认仅 `&TextParagraph` 路径在用);**不删** `ComponentHelperExt::copy`(已核实有调用方)

## 9. 文档与知识库

- [x] 9.1 `dev-notes/knowledge/hooks-and-state.md`:补 `try_*` 非阻塞/重入快速 panic 语义、ReactiveHandle 单一真源、依赖比较改 PartialEq;明确 State/Atom 保留 SyncStorage+Send+Sync 支持后台写入
- [x] 9.2 `dev-notes/knowledge/macros-and-props.md`:更新 AnyProps 借用生命周期收紧后的 unsafe 契约;清理 `parse_head` 等过时描述
- [x] 9.3 `dev-notes/knowledge/runtime-architecture.md`:补透明布局空子树重置、calc_children_areas 区域数契约、poll_change 三路全 poll 不变量

## 10. 验证

- [x] 10.1 四件套全绿(`--all-features`):`cargo test`、`cargo clippy -- -D warnings`、`cargo fmt --check`、`RUSTDOCFLAGS="-D warnings" cargo doc`
- [x] 10.2 本变更新增测试全通过:借用逃逸 trybuild、`with_layout_style` 误用 trybuild、路由动态尾段边界、ScrollView harness
- [x] 10.3 `cargo run --example` 关键示例行为正常:router / scrollview / store / input / counter / control_flow
- [x] 10.4 grep 收敛:无 `try_read` 自旋 `loop` 残留、无 `use_route_mut` 残留、`DropRowImpl` 已更名、`ComponentHelperExt::copy` 仍在
