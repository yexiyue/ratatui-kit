## Why

`add-test-suite` 延后了组件渲染测试，因为驱动组件树渲染需要 `update`，而 `update` 经 `dyn ComponentHelperExt::update_component` 间接持有 `Terminal<CrossTerminal>`（构造需真实 TTY），无法在无头测试里运行。泛型化 `update` 会破坏 `dyn` 对象安全。本变更对终端做**对象安全的类型擦除**，从而解锁离屏渲染 harness 与组件渲染测试。

## What Changes

- 抽出对象安全 trait（暂名 `UpdaterTerminal`），仅暴露 `update` 路径真正用到的两项能力：
  - `insert_before`：把泛型闭包参数 **box 化**（`Box<dyn FnOnce(&mut Buffer)>`）以满足对象安全；
  - `events`：返回 `TerminalEvents<crossterm::event::Event>`（**固定 Event 类型**——`use_events` 本就硬编码 crossterm Event，`CrossTerminal::Event` 亦是）。
- `ComponentUpdater` 改持有 `&mut dyn UpdaterTerminal` 而非 `&mut Terminal<CrossTerminal>`，保持自身为**非泛型具体类型**（不破坏 `update_component` 的 `dyn` 分发）。
- `update` 路径同步改造：`InstantiatedComponent::update`、`ComponentUpdater::{new,terminal}`、`use_insert_before`、`use_events`、`Tree::render`（update 步传 `&mut dyn UpdaterTerminal`，draw 步仍用具体 `Terminal`）。
- 提供 test-only「单次离屏渲染元素到 `ratatui::backend::TestBackend` Buffer」harness：用 no-op `UpdaterTerminal` 跑一次 update + `ratatui::Terminal<TestBackend>` 跑 draw → 返回 `Buffer`。
- 补代表性组件渲染测试（`Text`/`Border`/`View`/`Center`）。

## Capabilities

### New Capabilities
- `component-render-tests`: 组件渲染测试契约——对象安全终端抽象（`update` 路径只依赖 `insert_before` + `events`）、单次离屏渲染 harness、用 `TestBackend` Buffer 断言组件输出、以及保持 `CrossTerminal` 运行时行为不变。

### Modified Capabilities
<!-- 无:openspec/specs/ 当前为空。 -->

## Impact

- **代码**：`terminal/mod.rs`（新 trait + 对 `Terminal<CrossTerminal>` 的 impl + 空 `TerminalEvents` 构造）、`render/updater.rs`、`component/instantiated_component.rs`、`render/tree.rs`、`hooks/use_insert_before.rs`、`hooks/use_events.rs`；新增渲染 harness + `Text`/`Border`/`View`/`Center` 渲染测试。
- **公开 API**：`Terminal`/`CrossTerminal` 仍公开，内部多一层 trait；`ComponentUpdater::terminal()` 返回类型由 `&mut Terminal` 变为 `&mut dyn UpdaterTerminal`（**可能影响**直接调用该方法的外部代码——评估为极低概率，仅自定义 hook 会用）。
- **依赖**：复用 ratatui 自带 `TestBackend`，无新增运行时依赖。
- **风险**：中——改 `update` 核心路径。须保证 `CrossTerminal` 行为不变（examples 冒烟 + 现有 23 单测 + trybuild 全绿）。
- **依赖关系**：建立在 `add-test-suite` 之上（补齐其延后的第 5 组任务）。
