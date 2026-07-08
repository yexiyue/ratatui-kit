## ADDED Requirements

### Requirement: 依赖比较可靠,无 hash 碰撞

`use_memo`/`use_async_effect` 判断依赖是否变化 SHALL 基于依赖值的相等比较(`PartialEq`),MUST NOT 仅以依赖的 `u64` 哈希作为唯一判据(哈希碰撞会导致漏更新)。

#### Scenario: 不同依赖即使哈希碰撞也重算
- **WHEN** 两组不同的依赖值先后传入(若仅看 `u64` 哈希会碰撞)
- **THEN** 依赖变化被正确识别,memo 重新计算 / effect 重新执行

### Requirement: 依赖型 hook 首次必跑

`use_memo`/`use_effect`/`use_async_effect` 在组件首帧 SHALL 必定执行一次(计算或副作用),与依赖的具体值无关。

#### Scenario: 首帧依赖恰为初始哨兵值也执行
- **WHEN** 组件首帧的依赖比较值恰好等于结构体默认/初始值(如旧实现 `deps_hash` 默认 0、而首帧依赖哈希也为 0)
- **THEN** 首次计算/副作用仍然执行,不被跳过

### Requirement: 异步副作用完成触发收尾帧

`use_future`/`use_async_effect` 的 future 完成时 SHALL 触发一次重渲(其 `poll_change` 在 future 刚就绪的那次返回 `Ready`),以反映完成后的状态。

#### Scenario: future 完成后 UI 刷新一次
- **WHEN** `use_async_effect`/`use_future` 的 future 完成,且其副作用仅读不写响应式状态
- **THEN** 渲染循环被唤醒并重渲一帧,UI 不停留在完成前的倒数第二帧状态
