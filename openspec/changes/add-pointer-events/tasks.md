# Tasks

## 1. 终端捕获与显式启用（D1）

- [x] 1.1 新增 `RunOptions { mouse: bool }`（默认 `false`），贯穿 `ElementExt::fullscreen_with` / `render_loop_with` → `render_loop` → `Tree::new` → `SystemContext` 挂 `mouse_enabled` 标志；验证：现有 `fullscreen()` / `render_loop()` 调用点零改动通过编译，`SystemContext` 单测覆盖标志读写
- [x] 1.2 启用时在终端 setup 执行 `EnableMouseCapture`，`RestoreGuard` drop 时先 `DisableMouseCapture` 再 `ratatui::restore()`；验证：`cross_terminal` / `tree` 单测（setup 分支选择、guard 存在性）+ 手动运行示例后终端原生拖选恢复
- [x] 1.3 指针类注册在 `mouse_enabled == false` 时 `debug_assert!` 并输出包含 `fullscreen_with(RunOptions { mouse: true, .. })` 的诊断信息；验证：`#[should_panic]` 单测（debug 构建生效）

## 2. 分发核心：Stamp 回填与冒泡排序（D2、D3）

- [x] 2.1 将 handler 共享句柄 `Rc<Cell<Rect>>` 升级为 `Rc<Cell<Stamp>>`（`area` / `frame` / `draw_seq`），`begin_frame` 递增帧号，`pre_component_draw` 一次性盖章，`dispatch` 帧号不匹配当前帧视作未命中；同步迁移 `use_size` 之外的既有 `Rc<Cell<Rect>>` 使用方（ScrollView 注册路径）；验证：`input` 单测——「未绘制帧的旧区域不命中」「挂载后从未绘制不命中」
- [x] 2.2 鼠标事件层内排序 tie-break 从 `order asc` 改为 `draw_seq desc`（键盘事件保持 `(z, priority, order)` 不变）；验证：`input` 单测——子 Ignored 冒泡到父、子 Consumed 截断、重叠兄弟后画者先、键盘分发序回归用例全部通过
- [x] 2.3 为「遮罩点击关闭」组合写集成级单测：全屏 scrim handler + 内层面板 handler，点击面板消费区不触发 scrim、点击空白冒泡触发 scrim；验证：该测试在 2.2 排序下通过

## 3. 指针捕获路由（D4）

- [x] 3.1 hook 侧铸造跨帧稳定 `PointerToken`，`HandlerEntry` 携带；`InputRuntime` 在 hit_test handler 消费 `Down(Mouse)` 时记录 `(token, button)`，该按钮后续 `Drag` / `Up` 绕过命中过滤与排序独占直送，`Up` 清除；捕获期间 token 无对应 handler（组件卸载）则自动失效；验证：`input` 单测——拖出区域持续接收、捕获期间其他 handler 隔离、Up 后恢复传播、卸载自动失效、双按键各自独立

## 4. hover 全局 diff 与重绘抑制（D6）

- [x] 4.1 `InputRuntime` 维护 hover 目标集合：`Moved` 事件依据上帧 Stamp 集中计算最内命中链并更新各 token 布尔；`use_hover()` 返回当前值；验证：`input` / hooks 单测——移入置真、移出置假、区域内移动不变
- [x] 4.2 `dispatch` 返回「是否需要重绘」，`Moved` 无人消费且 hover 集合无变化时 `render_loop` 跳过本轮 render 直回 `select`；验证：单测——跳过路径不调用 draw；handler 消费或写 State（waker 唤醒）时仍重绘
- [x] 4.3 当帧存在 hover / Move 追踪注册时惰性执行 `1003h`（只升不降，经终端抽象执行）；验证：单测覆盖「存在注册才置升级标志」+ 手动示例确认不支持终端自然降级（无 hover 事件、其余正常）

## 5. 语义化 hooks（D5、D8）

- [x] 5.1 定义 `PointerEvent { kind, button, position, modifiers, area }` 与 `PointerKind`（Down / Up / Click / Drag / Move / Enter / Leave / Scroll 四向），实现 `use_pointer(PointerOptions, impl FnMut(PointerEvent) -> EventResult)` 包装 raw `Event::Mouse`；验证：单测——kind 映射、自控消费（放行时冒泡给外层）
- [x] 5.2 实现 `use_click`：Down+Up 同目标合成 `Click { count, .. }`，300ms 双击窗口计数递增（上限 3），跨目标不合成，处理后默认 `Consumed`；验证：单测——单击 / 双击 / 三击 / 跨目标 Down-Up / 消费不冒泡
- [x] 5.3 实现 `use_drag`：基于捕获路由输出 `DragUpdate { phase: Start | Move | End, from, to, button, modifiers }`；验证：单测——完整三态序列、拖出区域仍收到 Move / End

## 6. Button 组件（D7）

- [x] 6.1 实现 `ButtonProps { label, on_press: Handler<'a, ()>, variant, disabled }` 与渲染（variant 样式对接 theme-system、disabled 灰显、hover 高亮，disabled 不注册命中）；验证：TestBackend 渲染测试（normal / hover / disabled / 各 variant 快照，参照 component-render-tests 惯例）
- [x] 6.2 接线 `on_press`：左键 `Down` 命中即触发并消费；验证：集成测试——区域内按下触发一次、父级 handler 不收到、disabled 不触发、区域外按下不触发

## 7. 示例、文档与集成验证

- [x] 7.1 新增 `examples/mouse`：按钮（含 disabled）、遮罩点击关闭、可拖拽分隔条（use_drag）、hover 高亮区四类交互；验证：`cargo run --example mouse` 手动走查四类交互 + README/PR 附终端截图
- [x] 7.2 更新 `skills/ratatui-kit/references/`（events-state-routing.md 补指针冒泡与捕获、hooks.md 补四个新 hook）与 `examples/` 索引；验证：文档中的 API 签名与实现一致（编译级核对）
- [x] 7.3 全量验证矩阵：`cargo test --locked --all-features --workspace --lib --tests --examples`、`cargo clippy --all-targets --all-features --workspace -- -D warnings`、`cargo fmt --all --check`、`RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items --all-features --workspace --examples` 全部通过
