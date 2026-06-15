## 1. 测试基础设施

- [x] 1.1 `packages/ratatui-kit/Cargo.toml` 加 `[dev-dependencies] trybuild`
- [x] 1.2 新增 `packages/ratatui-kit/tests/ui.rs` harness（`pass("tests/ui/pass/*.rs")` + `compile_fail("tests/ui/fail/*.rs")`）

## 2. 宏通过用例（tests/ui/pass/）

- [x] 2.1 `element!`：控制流（if/if let/else if/for/match，含分支返回不同元素类型）、`{ expr }`、`Text(text:)`（`tests/ui/pass/dsl.rs`）
- [~] 2.2 `element!` 适配器：`widget(expr)` 已覆盖；`stateful(widget, state)` 的 pass 用例待补（需配 StatefulWidget + State）
- [~] 2.3 `#[component]`：函数组件已覆盖（dsl.rs 的 `App`）；无 props 用 NoProps 的显式用例待补
- [~] 2.4 `#[derive(Props)]` / `#[with_layout_style]`：经 dsl.rs 用到的 View/Border/Text 间接覆盖；专用 pass 用例待补
- [ ] 2.5 `routes!`：含嵌套子路由与动态参数 `/:id`（`router` feature，放 `tests/ui/pass_full/`，harness 加门控）
- [ ] 2.6 `use_stores!` 与 `#[derive(Store)]`（`store` feature，同上）

## 3. 宏失败用例（tests/ui/fail/ + .stderr）

- [x] 3.1 旧 `$` 适配器语法 → 迁移提示文案（`old_dollar_syntax`）
- [x] 3.2 旧 `#(expr)` 子节点语法 → 迁移提示文案（`old_hash_syntax`）
- [x] 3.3 `widget(a, b)` 多参 / `stateful(a, b, c)` 多参 → 本库 `syn::Error` 文案
- [x] 3.4 `#[component]` 非法参数名 → 本库报错（`bad_component_param`）
- [x] 3.5 生成 `.stderr` 并复核：5 个用例均只含本库稳定文案（已加 `#![allow(unused)]` 去除 import 警告噪音）

## 4. 运行时逻辑单测（各模块 #[cfg(test)]）

- [x] 4.1 `element/key.rs`：`ElementKey::decl`/`user` 不碰撞、相等性、Hash 与 Eq 自洽
- [x] 4.2 `multimap.rs`：FIFO pop_front、缺失 key、iter 只产未移除项
- [x] 4.3 `hooks/use_state.rs`：`State<T>` 的 `+=`/`-=`/`*=`、`set`/`get`、`Copy` 句柄共享
- [x] 4.4 `store/`（`store` feature）：`StoreState` 运算符（+=/-=）、set/get、Copy 共享
- [x] 4.5 `router/history.rs`：扩充 back/forward/go 越界用例
- [x] 4.6 `router` 路径匹配单测：把匹配逻辑从 Outlet 抽成 `Route::match_path`（可测），覆盖动态参数提取、不跨 `/`、静态段边界、根路由不命中、无匹配

## 5. 组件渲染 harness 与渲染测试（⏸ 延后：需核心改动）

> 阻塞点：`InstantiatedComponent::update` 经 `dyn ComponentHelperExt::update_component` 间接持有
> `Terminal<CrossTerminal>`,而构造它需真实 TTY。泛型化会破坏 `dyn` 对象安全;须把终端抽象做
> **对象安全的类型擦除**(如把 `insert_before` 的闭包 box 化、抽出 object-safe `TerminalHandle`),
> 属独立核心改动,单列一个 change 更稳妥。

- [ ] 5.1 落地「单次离屏渲染元素到 `ratatui` Buffer」test-only harness（design 决策 3 的 A/B，倾向 B）
- [ ] 5.2 `Text` 渲染断言
- [ ] 5.3 `Border` 边框断言
- [ ] 5.4 `View`/`Center` 布局/居中断言
- [ ] 5.5 （可选）`Modal`/`ScrollView`

## 6. 收尾

- [x] 6.1 更新 `dev-notes/knowledge/toolchain.md`：测试约定改为「编译验证为基线 + 宏/运行时针对性测试」，并记录渲染 harness 的阻塞点
- [x] 6.2 四件套全绿（`--all-features`）：运行时单测 23（含 router/store）+ trybuild ui 已 clippy/fmt/test/doc 全绿
