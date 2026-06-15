# component-render-tests Specification

## Purpose
TBD - created by archiving change render-test-harness. Update Purpose after archive.
## Requirements
### Requirement: 终端在 update 路径以对象安全接口暴露

`update` 路径访问终端 SHALL 通过一个对象安全 trait（仅 `insert_before` 与 `events` 两项能力），而非具体 `Terminal<CrossTerminal>`。`ComponentUpdater` MUST 保持非泛型具体类型（以不破坏 `update_component` 的 `dyn` 分发），改持有 `&mut dyn` 该 trait。

#### Scenario: ComponentUpdater 不绑定具体终端
- **WHEN** 审视 `ComponentUpdater` 持有的终端
- **THEN** 其类型为 `&mut dyn`（对象安全 trait），可由真实 `CrossTerminal` 或测试 no-op 终端满足

#### Scenario: insert_before 闭包 box 化
- **WHEN** `use_insert_before` 经该 trait 调用 `insert_before`
- **THEN** 闭包以 `Box<dyn FnOnce(&mut Buffer)>` 传入，使该方法对象安全

#### Scenario: events 固定为 crossterm Event
- **WHEN** `use_events` 经该 trait 订阅事件
- **THEN** 返回 `TerminalEvents<crossterm::event::Event>`（与 `use_events`/`CrossTerminal` 既有的 Event 类型一致）

### Requirement: CrossTerminal 运行时行为不变

抽象化 MUST NOT 改变真实 `CrossTerminal` 下的运行时行为：事件分发、`insert_before`、渲染输出与改造前一致。

#### Scenario: 现有示例与测试不回归
- **WHEN** 抽象化落地后跑 examples 与既有 23 单测 + trybuild
- **THEN** 全部通过，渲染/交互行为与改造前一致

### Requirement: 单次离屏渲染 harness

SHALL 提供 test-only 入口，把一个元素的组件树跑一次 `update`（用 no-op 终端）+ 一次 `draw`（用 `ratatui::backend::TestBackend`），返回渲染后的 `Buffer`。

#### Scenario: 渲染返回 Buffer
- **WHEN** 用 harness 在固定尺寸上渲染一个元素
- **THEN** 返回该尺寸的 `Buffer`，其内容为组件一次渲染的输出

### Requirement: 代表性组件有渲染断言

SHALL 用 harness 对代表性组件渲染并断言 Buffer 内容，至少覆盖 `Text`、`Border`、`View`、`Center`。

#### Scenario: Text 渲染文本
- **WHEN** 渲染 `Text(text: "hi")`
- **THEN** Buffer 对应位置出现 `hi`

#### Scenario: Border 绘制边框
- **WHEN** 渲染带边框的 `Border`，内含一个子元素
- **THEN** Buffer 四周出现边框字符，子内容落在内部区域

#### Scenario: Center 居中子内容
- **WHEN** 渲染 `Center` 包裹一段文本
- **THEN** 文本出现在 Buffer 的居中区域

