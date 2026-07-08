# scroll-view-layout Specification

## Purpose
TBD - created by archiving change harden-audit-findings. Update Purpose after archive.
## Requirements
### Requirement: 子区域尺寸计算无整数溢出

`ScrollView` 推导内容/子区域尺寸时 MUST NOT 因 `Fill`、大 `Percentage`、`Ratio` 约束产生 `u16` 整数溢出;尺寸计算 SHALL 在更高位宽进行并饱和收口到合法范围(优先直接复用 `ratatui::Layout` 推导子区域尺寸,以正确处理弹性约束语义)。

#### Scenario: 大 Fill 因子不 panic
- **WHEN** ScrollView 子节点使用 `Fill(820)` 之类的大权重,终端宽 80(`80 * 820` 超 `u16::MAX`)
- **THEN** 尺寸推导不溢出 panic(debug)也不回绕错乱(release),得到合法尺寸

#### Scenario: 负 gap 不回绕
- **WHEN** `LayoutStyle.gap` 为负且子节点数 ≥ 2
- **THEN** 间隙累加饱和到非负,不产生 `as u16` 回绕导致的 panic 或超大缓冲分配

### Requirement: 滚动条显隐用单一公式与单一尺寸来源

决定"是否预留滚动条并缩小内容区"的逻辑 SHALL 由单一函数、基于同一内容尺寸来源计算;`calc_children_areas` 与渲染滚动条两处 MUST NOT 使用互相漂移的两套公式(是否 +1、用估算尺寸还是真实缓冲尺寸)。

#### Scenario: 临界尺寸不错配
- **WHEN** 内容尺寸与视口接近相等(如垂直方向内容高 = 视口高 - 1)
- **THEN** "是否预留滚动条/缩小内容区"在布局与渲染两处判定一致,滚动条不覆盖内容、也不预留出导致内容缺失的空行

### Requirement: 滚动 hook 逐帧同步受控 props

`ScrollView` 的滚动 hook SHALL 每帧把当前 props(`scroll_view_state`、是否含 `block`)同步进 hook 内部状态,MUST NOT 仅在首帧初始化后固定不变。

#### Scenario: 运行中切换受控/block 生效
- **WHEN** 运行时把 `scroll_view_state` 从 `None` 切到 `Some`、调换 `State` 句柄,或开关 `block`
- **THEN** 渲染路径与事件路径使用同一份最新 props,滚动状态与滚动条不错位

