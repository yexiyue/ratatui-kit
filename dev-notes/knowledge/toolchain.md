# Toolchain（构建 / 特性门控 / 发布）

## 概览

本主题覆盖 ratatui-kit 这个 Cargo workspace 的工程化约束：cargo 命令矩阵、**feature flags 门控带来的「改了模块却没编译到」陷阱**、lefthook/CI 一致性、release.sh 发布流程，以及「仓库无单元测试」这一非显然约定。新增依赖、改 feature、动 CI/发布脚本前先读本文件。

## 命令矩阵

### 四件套必须用 `--all-features` 跑

CLI 与 CI/lefthook 完全对齐的四条命令（提交前 lefthook 会自动跑前三 + docs）：

```bash
cargo test --locked --all-features --workspace --lib --tests --examples
cargo clippy --all-targets --all-features --workspace -- -D warnings
cargo fmt --all --check                       # rustfmt.toml: tab_spaces=4
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items --all-features --workspace --examples
```

**正确做法**：本地验证一律带 `--all-features`。主库默认 **不启用任何 feature**，缺了它 `router`/`store`/`input`/`tree` 门控的模块根本不参与编译，clippy/test 会「假绿」。

**不要做**：用裸 `cargo build` / `cargo clippy` 验证改动——会漏掉所有特性门控模块的报错。

**相关文件**：`lefthook.yaml`、`.github/workflows/CI.yaml`

### docs 命令的 `RUSTDOCFLAGS` 差异

`RUSTDOCFLAGS="-D warnings"` 只在 **CI**（`.github/workflows/CI.yaml` 的 docs job）设置；`lefthook.yaml` 的 docs job 未带该环境变量。本地复现 CI 的文档失败时务必手动加上前缀，否则 doc warning 在本地不报错而 CI 红。

**相关文件**：`.github/workflows/CI.yaml`、`lefthook.yaml`

## Feature flags 门控

### 改门控模块必须开对应 feature 才编译得到

主库 `default = []`，高级组件全部按 feature 门控。映射关系：

| feature | 解锁内容 | 额外依赖 |
|---|---|---|
| `router` | `RouterProvider`/`Outlet`、`routes!`、`use_router`/`use_navigate` | `regex` |
| `store` | `StoreState`、`#[derive(Store)]`、`use_stores!` | — |
| `input` | `Input` 组件 | `tui-input` |
| `tree` | `TreeSelect` 组件 | `tui-tree-widget` |
| `full` | 上述全部 | — |

宏库 `ratatui-kit-macros` 有**独立**的 `router`/`store` feature，由主库同名 feature 透传（见主库 `Cargo.toml` 的 `ratatui-kit-macros/router` 写法）。改宏库里 router/store 相关代码时，主库侧的透传也要同步。

**正确做法**：改 `src/components/router/`、`src/store/`、`src/components/input.rs` 等模块时，用 `--all-features` 或 `--features <name>` 编译；新增门控模块要在 `lib.rs` / `components/mod.rs` 加 `#[cfg(feature = "...")]`，并在 `full` 聚合里登记。

**相关文件**：`packages/ratatui-kit/Cargo.toml`、`packages/ratatui-kit/src/lib.rs`、`packages/ratatui-kit/src/components/mod.rs`

### `textarea` 特性已随 ratatui 0.30 迁移下线

`tui-textarea`（最新 0.7.0 钉死 ratatui ^0.29）无 0.30 兼容版，故 `textarea` 的 feature / 依赖 / example 已移除。组件源码隔离保留在 `src/components/textarea.rs`（**未声明为模块、未接入树**），example 改名为 `examples/textarea.rs.disabled`。

**正确做法**：待 `tui-textarea` 发布 0.30 兼容版后，恢复 feature/依赖/example 并在 `components/mod.rs` 重新 `pub mod textarea;`。todo.md 另有「用 tui-input 重写 textarea 以支持自动换行」的方案——动手前先确认走哪条路。

**相关文件**：`packages/ratatui-kit/src/components/textarea.rs`、`packages/ratatui-kit/Cargo.toml`、`todo.md`

## 测试与发布

### 仓库没有单元测试——`cargo test` 验证的是「能否编译」

当前 workspace **没有 `#[test]` 单元测试**。`cargo test ... --examples` 的真实作用是确保所有 example 和 doctest 能编译通过。

**正确做法**：把「示例能编译 + 文档能编译」当作回归基线；新增组件优先补一个 `examples/` 下的可运行示例，而非纠结单测覆盖。改了公开 API 后跑一遍 examples 即是冒烟测试。

**相关文件**：`examples/`、`CLAUDE.md`

### 发布走 release.sh + tag 触发 CD

`release.sh [level] [exclude-crate ...]` 用 `cargo release` 逐 crate 升版本 + git-cliff 生成 CHANGELOG，最后 `git push origin main --tags`。tag 形如 `<crate>-v<version>`，由 `.github/workflows/CD.yaml`（`on: tags '*-v*'`）触发 `cargo publish packages/<crate>`。

**正确做法**：三个 crate（`ratatui-kit-macros` / `ratatui-kit` / `ratatui-kit-examples`）版本相互独立，按需用 `--exclude` 或交互式选择跳过不发布的 crate。`ratatui-kit-examples` 通常不发布到 crates.io。

**不要做**：手动 `cargo publish` 绕过脚本——会漏掉 CHANGELOG 生成与 tag 约定，导致 CD 的 release notes 抽取（按 CHANGELOG 段落 awk）拿到空内容。

**相关文件**：`release.sh`、`release.toml`、`.github/workflows/CD.yaml`
