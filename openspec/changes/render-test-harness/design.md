## Context

调查结论（决定方案的关键事实）：

- `update` 路径对终端**实例**的调用只有两处：`use_insert_before` 的 `updater.terminal().insert_before(height, F)` 与 `use_events` 的 `updater.terminal().events()`。`use_size` 用的是自由函数 `terminal::size()`，不碰实例。
- `ComponentUpdater` 持有 `terminal: &mut Terminal<CrossTerminal>`，并被传入 `dyn ComponentHelperExt::update_component`。**`update_component` 是 `dyn`，故 `ComponentUpdater` 必须保持非泛型**——不能把终端泛型化，只能让它持有一个**对象安全 trait 对象**。
- `insert_before<F: FnOnce(&mut Buffer)>` 因泛型 `F` 非对象安全；`events()` 返回 `TerminalEvents<T::Event>` 因关联类型 `Event` 非对象安全。
- 但 `use_events` 的回调 **硬编码 `crossterm::event::Event`**，`CrossTerminal::Event` 也是它——所以可把 trait 的 `events()` **固定为 `TerminalEvents<crossterm::event::Event>`**，不丢通用性（框架本就 crossterm-only）。
- `draw` 步独立于 `update`（`Tree::render` 里 `terminal.draw(|frame| ...)`），测试可换用 `ratatui::Terminal<TestBackend>` 取 `Frame`。

## Goals / Non-Goals

**Goals:**
- 让 `update` 可在无头（无真实 TTY）下运行，从而离屏渲染组件。
- 保持 `CrossTerminal` 运行时行为与对象安全（`dyn update_component`）不变。
- 提供可复用的「渲染元素到 Buffer」test harness + 组件渲染断言。

**Non-Goals:**
- 不把整套 `Terminal`/`TerminalImpl` 重写为多后端框架——只擦除 `update` 路径所需的最小面。
- 不测试异步事件循环、`use_future`、真实事件分发。
- 不追求覆盖所有组件，代表性即可。

## Decisions

### 决策 1：对象安全 `UpdaterTerminal` trait，ComponentUpdater 持 `&mut dyn`

```rust
pub(crate) trait UpdaterTerminal {
    fn insert_before(&mut self, height: u16, draw_fn: Box<dyn FnOnce(&mut Buffer)>) -> io::Result<()>;
    fn events(&mut self) -> io::Result<TerminalEvents<crossterm::event::Event>>;
}
```

- `ComponentUpdater.terminal: &'a mut dyn UpdaterTerminal`；`ComponentUpdater::terminal()` 返回 `&mut dyn UpdaterTerminal`。ComponentUpdater 仍是具体类型 → `update_component` 仍对象安全。
- `insert_before` 把闭包 box 化(满足对象安全)；`events()` 固定 crossterm Event(见 Context)。
- `impl<T: TerminalImpl<Event = crossterm::event::Event>> UpdaterTerminal for Terminal<T>`：覆盖 `CrossTerminal`，其 `insert_before` 转发到内部泛型版本(`Box<dyn FnOnce>` 本身即 `FnOnce`)，`events()` 转发现有实现。

### 决策 2：update 路径换抽象，draw 路径保持具体

- `InstantiatedComponent::update(terminal: &mut dyn UpdaterTerminal, ...)`；其递归 `component.update(self.terminal, ...)` 透传同一 `&mut dyn`。
- `Tree::render(terminal: &mut Terminal)`：**update 步**把 `terminal` 作 `&mut dyn UpdaterTerminal` 传下；**draw 步**仍用具体 `terminal.draw(...)`。两步解耦,正好让测试分别替换。
- `use_insert_before`：`updater.terminal().insert_before(height, Box::new(callback))`。
- `use_events`：`updater.terminal().events()`(类型已对齐)。

### 决策 3：渲染 harness —— no-op 跑 update + TestBackend 跑 draw

```rust
// test-only(#[cfg(test)] 或 pub(crate))
fn render_to_buffer(el: impl Into<AnyElement<'static>>, w: u16, h: u16) -> ratatui::buffer::Buffer {
    let mut el = el.into();
    let helper = el.helper();
    let mut tree = Tree::new(el.props_mut(), helper);

    // 1) update:用 no-op 终端(insert_before 空操作、events 返回空 TerminalEvents)
    let mut noop = NoopTerminal::default();
    tree.update_once(&mut noop);

    // 2) draw:ratatui TestBackend 取 Frame → ComponentDrawer → 树 draw
    let mut term = ratatui::Terminal::new(ratatui::backend::TestBackend::new(w, h)).unwrap();
    term.draw(|frame| {
        let area = frame.area();
        let mut drawer = ComponentDrawer::new(frame, area);
        tree.draw_root(&mut drawer);
    }).unwrap();

    term.backend().buffer().clone()
}
```

- 需把 `Tree` 暴露两个 `pub(crate)` 测试入口：`update_once(&mut dyn UpdaterTerminal)`(只跑 update 步) 与 `draw_root(&mut ComponentDrawer)`(只跑 draw 步)。
- `NoopTerminal`：`insert_before` 返回 `Ok(())` 不绘制；`events()` 返回**空** `TerminalEvents`(需给 `TerminalEvents` 加 `pub(crate)` 空构造,内部 `pending` 空、`waker` None)。
- 只渲染静态输出,不轮询 future/事件。

### 决策 4：组件渲染测试

用 harness 对 `Text`/`Border`/`View`/`Center` 渲染后断言 `Buffer`(按单元格内容/边框字符/落位)。这同时成为 `reduce-component-boilerplate`(②) 之类组件改动的回归网。

## Risks / Trade-offs

- **[改 update 核心路径]** → 改动面：updater/instantiated_component/tree/terminal/两个 hook。每步跑 examples + 现有 23 单测 + trybuild 确认 CrossTerminal 不回归。
- **[`ComponentUpdater::terminal()` 返回类型变化]** → 公开方法签名变(`&mut Terminal` → `&mut dyn UpdaterTerminal`)。自定义 hook 若直接用它会受影响；评估极低概率,CHANGELOG 注明。可选：保留一个返回具体类型的内部访问器,公开的 `terminal()` 仅暴露 trait 对象。
- **[events 固定 crossterm Event]** → 放弃 `TerminalImpl::Event` 的泛型通用性。但框架本就 crossterm-only(`use_events` 已硬编码),无实际损失。
- **[harness 仅静态渲染]** → 不覆盖异步/事件驱动行为。明确为 Non-Goal。

## Migration Plan

1. `terminal/mod.rs`：加 `UpdaterTerminal` trait + 对 `Terminal<T: ..Event=crossterm Event>` 的 impl + `TerminalEvents` 空构造(`pub(crate)`)。
2. `ComponentUpdater`：字段与 `new`/`terminal()` 改 `&mut dyn UpdaterTerminal`；递归 update 透传。
3. `InstantiatedComponent::update` 签名改 `&mut dyn UpdaterTerminal`。
4. `use_insert_before` box 化闭包；`use_events` 适配(类型已对齐,基本无改)。
5. `Tree`：`render` 内部拆出 `update_once` + `draw_root` 两个 `pub(crate)` 入口(供 harness)；`render` 仍按原顺序调它们。
6. 落地 `render_to_buffer` harness + `NoopTerminal`。
7. 补 `Text`/`Border`/`View`/`Center` 渲染测试。
8. 全程 examples + 23 单测 + trybuild + 四件套验证。

回滚：改动集中在 update 路径与新增测试；逐步提交,出问题还原对应提交。

## Open Questions

- `Tree`/`render()` 拆 `update_once`/`draw_root` 是否够；若 `render` 里 update 与 draw 有共享中间态,需一并暴露——实现时核对。
- 公开 `terminal()` 返回 trait 对象是否对任何现有自定义 hook 造成破坏——全仓库仅 `use_insert_before`/`use_events` 用它,影响可控。
