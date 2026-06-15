## 1. 对象安全终端抽象

- [x] 1.1 `terminal/mod.rs`：`pub trait UpdaterTerminal { insert_before(Box<dyn FnOnce(&mut Buffer)>); events() -> TerminalEvents<crossterm::event::Event>; }`
- [x] 1.2 `impl<T: TerminalImpl<Event = crossterm::event::Event>> UpdaterTerminal for Terminal<T>`（转发固有 insert_before/events）
- [x] 1.3 给 `TerminalEvents` 加 `#[cfg(test)] pub(crate) fn empty()`（空流，供 no-op 终端）

## 2. update 路径换抽象

- [x] 2.1 `ComponentUpdater`：`terminal` 字段与 `new`/`terminal()` 改为 `&mut dyn UpdaterTerminal`；递归透传（隐式 reborrow）
- [x] 2.2 `InstantiatedComponent::update` 签名改 `terminal: &mut dyn UpdaterTerminal`
- [x] 2.3 `use_insert_before`：无需改——queue 已存 `Box<dyn FnOnce(&mut Buffer)>`，正是 trait 期望类型
- [x] 2.4 `use_events`：无需改——`events()` 返回类型与既有 `TerminalEvents<crossterm Event>` 对齐
- [x] 2.5 `Tree`：拆出 `pub(crate) update_once(&mut dyn UpdaterTerminal)` + `pub(crate) draw_root(&mut ComponentDrawer)`，`render` 顺序调用保持原行为
- [x] 2.6 examples + 现有单测 + trybuild + 四件套全绿，CrossTerminal 行为不回归

## 3. 渲染 harness

- [x] 3.1 `NoopTerminal`（impls `UpdaterTerminal`：insert_before Ok、events 空流）
- [x] 3.2 `render_to_buffer(el, w, h) -> Buffer`：no-op 跑 `update_once` + `ratatui::Terminal<TestBackend>` 跑 `draw_root` → 返回 Buffer（`src/render/harness.rs`，`#[cfg(test)]`）

## 4. 组件渲染测试

- [x] 4.1 `Text`：`Text(text: "hi")` → 断言首行 "hi"
- [x] 4.2 `Border`：断言左上角边框字符 + 子内容在边框内
- [x] 4.3 `View`：断言子内容渲染
- [x] 4.4 `Center`：显式尺寸 → 断言内容居中（列 > 0）

## 5. 收尾

- [x] 5.1 四件套全绿（`--all-features`）：clippy `-D warnings`/fmt/test（29 lib + trybuild）/doc
- [x] 5.2 回填 `add-test-suite` 第 5 组为已完成；`toolchain.md` 去掉「渲染 harness 阻塞」注记
