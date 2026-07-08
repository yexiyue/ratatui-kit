## ADDED Requirements

### Requirement: draw 的 IO 错误经 Result 传播

渲染循环对 `terminal.draw` 的失败 SHALL 经 `?`/`Result` 向上传播,MUST NOT 用 `expect` 把可恢复的 IO 错误升级为 panic。

#### Scenario: draw 失败返回 Err
- **WHEN** `terminal.draw` 返回 IO 错误
- **THEN** 渲染循环以 `Err` 返回该错误,而非 panic

### Requirement: 失败路径恢复终端

无论渲染循环正常返回、返回 `Err`、还是 panic,终端 SHALL 被恢复(退出 raw mode/alternate screen)。恢复 SHALL 由 guard/Drop 与 ratatui 的 panic hook 共同保证(panic 路径由 hook 兜底,正常与 Err 返回路径由 guard 兜底)。

#### Scenario: 错误返回也恢复终端
- **WHEN** 渲染循环因 draw 错误提前返回 `Err`
- **THEN** 终端在返回前被恢复,不残留破坏的终端状态

### Requirement: draw 阶段对可恢复数据错误不 panic

组件 `draw` 对可恢复的数据错误(如 `TreeSelect` 顶层重复 identifier 导致底层 `Tree::new` 返回 `Err`)MUST NOT `unwrap`/panic;构造与校验 SHALL 在 `new`/`update` 阶段完成并缓存结果,`draw` 仅消费缓存或渲染占位。

#### Scenario: 重复 identifier 不崩溃
- **WHEN** `TreeSelect` 收到含重复顶层 identifier 的数据
- **THEN** 不在每帧 `draw` 中 panic 拖垮整个 TUI,而是优雅处理(构造期去重/缓存,或渲染占位)
