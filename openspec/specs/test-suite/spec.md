# test-suite Specification

## Purpose
TBD - created by archiving change add-test-suite. Update Purpose after archive.
## Requirements
### Requirement: 宏编译测试覆盖全部公开宏

trybuild 编译测试 SHALL 覆盖全部公开宏：`element!`、`#[component]`、`#[derive(Props)]`、`routes!`、`use_stores!`、`#[derive(Store)]`、`#[with_layout_style]`。每个宏 MUST 至少有一个「正常用法编译通过」的用例。

#### Scenario: 每个宏都有通过用例
- **WHEN** 测试套件运行
- **THEN** 上述每个宏均存在一个编译通过的用例，覆盖其典型用法

#### Scenario: routes! 覆盖嵌套
- **WHEN** `routes!` 的通过用例
- **THEN** 覆盖含嵌套子路由与动态参数（`/:id`）的路由表能正常展开编译

### Requirement: 失败用例只断言本库稳定报错

宏失败用例 SHALL 仅断言由本库经 `syn::Error`/`input.error(...)` 主动产出的错误文案（旧 `$`/`#(expr)` 迁移提示、`widget`/`stateful` 参数错误、`#[component]` 非法参数名等）。MUST NOT 把 `.stderr` 断言建立在 rustc 自身类型错误文案上（跨版本不稳定）。

#### Scenario: 断言迁移提示
- **WHEN** 失败用例使用旧 `$` 或 `#(expr)` 语法
- **THEN** 编译失败且 stderr 含本库迁移提示文案

#### Scenario: 不绑定 rustc 文案
- **WHEN** 选取失败用例
- **THEN** 不选「stderr 主体由 rustc 类型检查生成」的场景作断言对象

### Requirement: 宏测试置于运行时 crate

宏的 UI 测试 SHALL 位于 `ratatui-kit` crate（`extern crate self as ratatui_kit` 使展开的 `::ratatui_kit::...` 可解析）。

#### Scenario: 展开路径可解析
- **WHEN** 一个宏通过用例被编译
- **THEN** 其展开引用的 `::ratatui_kit::...` 在该 crate 内解析成功

### Requirement: 运行时逻辑有针对性单测

运行时的可纯逻辑测试部分 SHALL 有 `#[test]` 覆盖，至少包括：`ElementKey`（`Decl`/`User` 不互相碰撞、`Hash`/`Eq` 自洽）、`use_state`/`store` 的运算符重载与 `Copy` 语义、`router` 路径匹配（段边界匹配、命名参数提取）、`history` 的越界行为。

#### Scenario: ElementKey 不碰撞
- **WHEN** 构造 `ElementKey::decl(k)` 与 `ElementKey::user(...)`
- **THEN** 两者不相等；相同输入相等、不同输入不等，`Hash` 与 `Eq` 一致

#### Scenario: State 运算符触发更新语义
- **WHEN** 对 `State<T>` 执行 `+= ` 等运算符重载
- **THEN** 值被更新（变更通知路径成立）

#### Scenario: 路由段边界匹配
- **WHEN** 测试路径匹配函数，输入 `/book-source-login` 与路由 `/book-source`
- **THEN** 不匹配（剩余 `-login` 非新段）

### Requirement: 代表性组件有渲染测试

SHALL 提供「单次离屏渲染到 `ratatui` `TestBackend` Buffer」的测试 harness，并用它对代表性组件（至少 `Border`、`Text`、`View`、`Center`）渲染后断言 Buffer 内容。

#### Scenario: Text 渲染内容
- **WHEN** 用 harness 在固定尺寸 Buffer 上渲染 `Text(text: "hi")`
- **THEN** Buffer 对应位置出现 `hi`

#### Scenario: Border 绘制边框
- **WHEN** 用 harness 渲染带边框的 `Border`
- **THEN** Buffer 四周出现边框字符，内容区在内部

### Requirement: 测试随 cargo test 运行且不改 CI

全部测试 SHALL 在现有 `cargo test --all-features --workspace --lib --tests --examples` 下运行；MUST NOT 需要改动 CI/lefthook 命令。

#### Scenario: CI 无改动即覆盖
- **WHEN** 现有 CI 测试命令执行
- **THEN** 单测、trybuild harness、组件渲染测试均被执行

### Requirement: 更新「无单元测试」约定

`dev-notes/knowledge/toolchain.md` 中「仓库无单元测试」的约定 SHALL 更新为「以编译验证为基线，并对关键宏/运行时逻辑/组件补针对性测试」。

#### Scenario: 约定与现实一致
- **WHEN** 测试套件落地后读 `toolchain.md`
- **THEN** 其测试约定描述与实际（存在 trybuild + 单测 + 渲染测试）一致

