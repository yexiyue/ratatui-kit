// 宏 UI 编译测试。
//
// - `pass/`：新 DSL 应编译通过的用例（核心，不依赖门控特性）。
// - `fail/`：应编译失败的用例，`.stderr` 只断言本库经 `syn::Error` 主动产出的**稳定**文案
//   （旧 `$`/`#()` 迁移提示、`widget`/`stateful` 参数错误、`#[component]` 非法参数名），
//   不绑定 rustc 自身的类型错误信息（跨版本不稳定）。
// - `pass_router/`：依赖 `router` 的用例，仅在对应特性开启时纳入。

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/pass/*.rs");
    t.compile_fail("tests/ui/fail/*.rs");
    // TODO(add-test-suite): router 门控宏(routes!)的 pass 用例待补,
    // 届时加 `#[cfg(feature = "router")] t.pass("tests/ui/pass_router/*.rs")`。
    // 注:全局状态已改 atom（Atom/use_atom，非宏），无需 trybuild 宏用例。
}
