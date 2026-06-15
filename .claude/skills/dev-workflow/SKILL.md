---
name: dev-workflow
description: |
  项目开发工作流技能。在以下场景自动调用：
  (1) 编写或修改任何 packages/*/src/ 或 examples/ 下的代码
  (2) 添加新依赖、改 feature flag 或修改配置文件
  (3) 完成一个 feature 或修复一个 bug
  触发关键词：组件开发、bug 修复、重构、新功能、依赖升级、feature 门控、宏改动、配置变更
---

# Dev Workflow — 项目开发工作流

## 工作流程

### 1. 开发前：加载相关知识

根据当前任务，读取 `dev-notes/knowledge/` 下的相关主题文件：

| 任务涉及 | 读取文件 |
|---|---|
| cargo 命令、feature flags 门控、lefthook/CI、发布(release.sh)、版本 | `dev-notes/knowledge/toolchain.md` |
| Element/Component、协调(reconciliation)、渲染循环、Waker 响应式、布局/透明布局 | `dev-notes/knowledge/runtime-architecture.md` |
| Hooks(顺序规则/自定义 Hook)、use_state、全局 store、State 运算符重载 | `dev-notes/knowledge/hooks-and-state.md` |
| 过程宏(element!/#[component]/#[with_layout_style] 等)、Props 类型擦除、SendBlock、AnyProps unsafe | `dev-notes/knowledge/macros-and-props.md` |

**读取方式**：使用 Read 工具读取对应文件，遵循其中记录的最佳实践和注意事项。

如果不确定读哪个，读取 `dev-notes/knowledge/` 目录列表，根据文件名判断；跨主题的改动（如「加一个带布局的门控组件」）可同时读多个。

### 2. 开发中：遵循最佳实践

同时参考以下通用 skill（如果与当前任务相关，自动调用）：

- `/rust-best-practices` — Rust 通用规范（所有权 vs clone、`Result` 错误处理、性能、文档与测试）
- `/rust-async-patterns` — Tokio、异步 trait、并发与取消；改渲染循环、`poll_change`、`use_future`、Waker 相关代码时尤其相关

**优先级**：项目知识库 > 通用 skill > Claude 自身知识。当项目知识库中有明确记录时，以项目知识库为准（例如 SendBlock、Hook 顺序规则、feature 门控这些项目特定约束优先于通用做法）。

### 3. 开发后：更新知识库

完成代码修改后，**检查是否产生了新的项目知识**：

**需要记录的内容**：
- 新引入的依赖及其正确用法（尤其非显性、易踩坑的）
- 发现的配置坑和 workaround（如某依赖与 ratatui 0.30 不兼容、某 feature 透传遗漏）
- 做出的架构决策及原因（如某组件为何重写 `calc_children_areas`）
- 与通用最佳实践不同的项目特定做法
- 解决的 bug 的根因（如果不明显的话）

**不需要记录的内容**：
- 代码本身能表达的东西（看代码就能懂）
- 通用编程知识（Rust 所有权规则、tokio 用法等，不特定于本项目）
- 临时性的调试信息
- git/CHANGELOG 能查到的东西

**更新方式**：
1. 判断属于哪个主题文件（toolchain / runtime-architecture / hooks-and-state / macros-and-props）
2. 追加新条目到对应文件的合适子域下
3. 如果现有主题都不合适，再创建新主题文件（合并优先于新建，3-4 个文件是甜蜜点）
4. 如果发现已有条目过时（如 textarea 特性恢复、store 重构落地），更新或删除它

**条目格式**：

```markdown
### 条目标题

简短描述做了什么、为什么这样做。

**正确做法**：
- 具体的代码模式或配置

**不要做**（如果有）：
- 错误的做法及原因

**相关文件**：`path/to/file`
```

### 4. 代码质量检查

开发完成后，运行 `/simplify` 检查代码质量。lint/format/typecheck 命令（**必须带 `--all-features`，否则门控模块不参与编译，会「假绿」**）：

```bash
cargo test --locked --all-features --workspace --lib --tests --examples
cargo clippy --all-targets --all-features --workspace -- -D warnings
cargo fmt --all --check                       # rustfmt.toml: tab_spaces=4
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items --all-features --workspace --examples
```
