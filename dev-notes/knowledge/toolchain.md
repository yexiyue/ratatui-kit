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

**正确做法**：本地验证一律带 `--all-features`。主库默认 **不启用任何 feature**，缺了它 `router`/`atom`/`input`/`tree` 门控的模块根本不参与编译，clippy/test 会「假绿」。

**不要做**：用裸 `cargo build` / `cargo clippy` 验证改动——会漏掉所有特性门控模块的报错。

**相关文件**：`lefthook.yaml`、`.github/workflows/CI.yaml`

### docs 命令的 `RUSTDOCFLAGS` 差异

`RUSTDOCFLAGS="-D warnings"` 只在 **CI**（`.github/workflows/CI.yaml` 的 docs job）设置；`lefthook.yaml` 的 docs job 未带该环境变量。本地复现 CI 的文档失败时务必手动加上前缀，否则 doc warning 在本地不报错而 CI 红。

**相关文件**：`.github/workflows/CI.yaml`、`lefthook.yaml`

### docs Mermaid 图表用显式 Astro 组件

文档站支持 Mermaid 流程图，但不做全站 Markdown 代码块自动扫描。需要图表的 MDX 页面显式导入 `docs/src/components/Mermaid.astro` 并传入 `chart` 字符串；组件内部按当前 Starlight 主题渲染 Mermaid SVG。

**正确做法**：
- 教程中的状态流、渲染流、事件流优先用 Mermaid 表达。
- 只在确实需要流程图的页面引入 Mermaid 组件，避免所有页面都加载大体积图表运行时。
- `Mermaid.astro` 的客户端脚本动态 `import('mermaid')`，让页面先加载约 2KB 的控制脚本，再按需拉取 Mermaid 本体；不要恢复顶层静态 `import mermaid from 'mermaid'`。
- 真实终端运行效果仍然用 VHS GIF，不用 Mermaid 或假截图替代。

**相关文件**：`docs/src/components/Mermaid.astro`、`docs/src/content/docs/tutorials/async-state.mdx`、`docs/package.json`

### docs 部署路径与 sitemap 配置

文档站通过 GitHub Pages 项目站点发布，URL 结构是 `https://yexiyue.github.io/ratatui-kit/`。Astro 配置需要同时设置 `site: "https://yexiyue.github.io"` 和 `base: "/ratatui-kit"`，否则 `@astrojs/sitemap` 会跳过 sitemap 生成，或生成不带项目路径的链接。

**正确做法**：
- `docs/astro.config.mjs` 保持 `site` 为 GitHub Pages origin、`base` 为仓库项目路径。
- README 和文档内的公开链接统一指向 `/ratatui-kit/...` 新文档结构，不再使用旧的 `ratatui-kit-website/docs/...`。

**相关文件**：`docs/astro.config.mjs`、`README.md`、`docs/README.md`

### VHS 录制必须显式设置彩色终端环境

Codex / CI shell 可能带 `TERM=dumb` 或 `NO_COLOR=1`。裸 ANSI `printf` 仍会显示颜色，但 ratatui/crossterm 会按环境降级，导致 example 真实运行有颜色，VHS GIF 却只剩灰度样式。

**正确做法**：
- 每个 `docs/tapes/*.tape` 在 `Set Width` / `Set Theme` 等设置之后、`Type` 之前写：
  `Env TERM "xterm-256color"`、`Env COLORTERM "truecolor"`、`Env NO_COLOR ""`。
- `Set` 指令必须保持在 `Env` 前面；VHS 会忽略出现在非设置命令之后的 late setting。
- 发现 GIF 缺色时，先用 `script` 抓原始输出确认是否有 `38;5` / `48;5` / `38;2` 这类 SGR 色彩码，再怀疑主题或 GIF 后处理。

**相关文件**：`docs/tapes/*.tape`、`.codex/skills/ratatui-docs-demo-loop/references/recording-tools.md`

### VHS 录制可能需要非沙箱环境

VHS 0.11.0 在受限沙箱里启动录制后端时可能在 `randomPort()` 崩溃；同一个 tape 在非沙箱环境可以正常生成 GIF。这属于录制工具运行环境问题，不代表 example 或 tape 一定有错。

**正确做法**：
- 先在普通沙箱运行 `vhs docs/tapes/<example>.tape`；如果出现 `randomPort()` / nil pointer 这类 VHS 后端崩溃，再按权限流程用非沙箱重跑同一个命令。
- 全量验证录制链时用 `set -e; for f in docs/tapes/*.tape; do vhs "$f"; done`，确保任意一个 tape 失败都会停住；录制会真实运行对应 `cargo run --quiet --example ...`。
- 录制成功后用 `ffmpeg` 抽取中间帧检查颜色、布局和滚动状态；不要只看文件存在。

**相关文件**：`docs/tapes/*.tape`、`docs/public/recordings/*.gif`

### 文档站采用渐进式学习路径

文档信息架构参考 React Learn 和 Vue Guide 的层次：先 Quick Start 建立最小手感，再进入 UI 描述、交互、状态、组件组合，最后才是高级逃生口和内部机制。

**正确做法**：
- `start/*` 和第一批 `tutorials/*` 讲“怎么跑起来”和“框架日常 80% 概念”。
- 侧边栏优先保持“学习路径 / 基础教程 / 参考手册”的阅读节奏；组件、核心模型、高级扩展属于 Reference，不要过早插进教程流。
- `components/*` 承载可组合内置组件，不把业务主题带进 API。
- `advanced/*` 放自定义 Hook、Provider、原生 widget 桥接等逃生口。
- `internals/*` 解释运行时细节，避免提前压到初学路径里。

**相关文件**：`docs/src/content/docs/start/index.mdx`、`docs/astro.config.mjs`

### examples 按学习路径分组但保持命令稳定

Cargo 只会自动发现顶层 `examples/name.rs` 或 `examples/name/main.rs`。文档重写后 examples 会按 `examples/start/`、`examples/hooks/`、`examples/components/`、`examples/advanced/`、`examples/apps/` 等学习路径分组；分组目录下的示例必须在根 `Cargo.toml` 用 `[[example]]` 显式登记。

**正确做法**：
- 对外命令保持稳定：`cargo run --example counter` 不随源码目录变化。
- 迁移时一次移动一个学习组，同时更新 docs 中的源码路径标题。
- VHS tape 的 `cargo run --quiet --example <name>` 不需要因为目录迁移而改变。

**相关文件**：`Cargo.toml`、`examples/start/hello_world.rs`、`examples/start/counter.rs`、`examples/hooks/async_state.rs`、`examples/hooks/atom_state.rs`、`examples/core/control_flow.rs`、`examples/routing/router.rs`、`examples/input/input_mutex.rs`、`examples/components/input.rs`、`examples/components/search_input.rs`、`examples/components/scrollview.rs`、`examples/components/wrapped_text.rs`、`examples/components/modal.rs`、`examples/components/confirm_modal.rs`、`examples/components/alert_modal.rs`、`examples/components/shortcut_info_modal.rs`、`examples/components/select.rs`、`examples/components/multi_select.rs`、`examples/components/tree_select.rs`、`examples/components/virtual_list.rs`、`examples/components/virtual_multi_select.rs`、`examples/advanced/custom_widget.rs`、`examples/advanced/custom_hook.rs`、`examples/advanced/custom_provider.rs`、`examples/apps/todo_app.rs`、`docs/src/content/docs/examples/index.mdx`

## Feature flags 门控

### 改门控模块必须开对应 feature 才编译得到

主库 `default = []`，会拉入额外依赖的能力按 feature 门控；基础布局、文本、滚动、弹窗和选择组合中的一部分属于核心。映射关系：

| feature | 解锁内容 | 额外依赖 |
|---|---|---|
| `router` | `RouterProvider`/`Outlet`、`routes!`、`use_router`/`use_navigate` | `regex` |
| `atom` | `Atom`、`AtomState`、`use_atom` | — |
| `input` | `Input`、`SearchInput` 和 `tui_input` re-export | `tui-input` |
| `tree` | `TreeSelect` 组件 | `tui-tree-widget` |
| `virtual-list` | `VirtualList` 虚拟列表组件 | `tui-widget-list` |
| `full` | 上述全部 | — |

宏库 `ratatui-kit-macros` 有**独立**的 `router` feature，由主库同名 feature 透传（见主库 `Cargo.toml` 的 `ratatui-kit-macros/router` 写法）。全局状态已改为纯主库 `atom` feature，不再有 store 宏或宏库透传。

**正确做法**：改 `src/components/router/`、`src/atom/`、`src/components/input.rs`、`src/components/search_input.rs` 等模块时，用 `--all-features` 或 `--features <name>` 编译；新增门控模块要在 `lib.rs` / `components/mod.rs` 加 `#[cfg(feature = "...")]`，并在 `full` 聚合里登记。

**相关文件**：`crates/ratatui-kit/Cargo.toml`、`crates/ratatui-kit/src/lib.rs`、`crates/ratatui-kit/src/components/mod.rs`

### `textarea` 特性已随 ratatui 0.30 迁移下线

`tui-textarea`（最新 0.7.0 钉死 ratatui ^0.29）无 0.30 兼容版，故 `textarea` 的 feature / 依赖 / example 已移除。组件源码隔离保留在 `src/components/textarea.rs`（**未声明为模块、未接入树**），example 改名为 `examples/textarea.rs.disabled`。

**正确做法**：待 `tui-textarea` 发布 0.30 兼容版后，恢复 feature/依赖/example 并在 `components/mod.rs` 重新 `pub mod textarea;`。todo.md 另有「用 tui-input 重写 textarea 以支持自动换行」的方案——动手前先确认走哪条路。

**相关文件**：`crates/ratatui-kit/src/components/textarea.rs`、`crates/ratatui-kit/Cargo.toml`、`todo.md`

## 测试与发布

### 测试约定：编译验证为基线 + 宏/运行时/组件的针对性测试

**编译即基线**：`cargo test ... --examples` 仍以「所有 example + doctest 能编译」为回归底线；新增组件优先补一个可运行 example。

**已有的针对性测试**（`add-test-suite` 起逐步补齐，跑在 `cargo test --tests/--lib` 下）：

- **运行时单测**（各模块 `#[cfg(test)] mod tests`）：`element/key.rs`（ElementKey 不碰撞/Hash/Eq）、`multimap.rs`、`hooks/use_state.rs` 与 `atom/`（运算符重载/Copy/读写、Atom 惰性初始化、use_atom 订阅清理）、`components/router/{history,mod}.rs`（history 越界、`Route::match_path` 段边界与参数提取）。可在模块内经 `UseStateImpl::new`/`AtomState::new`/`Route::new` 构造被测对象。
- **宏 UI 测试**（`crates/ratatui-kit/tests/ui.rs` + `tests/ui/{pass,fail}/`，trybuild）：pass 验证新 DSL 编译通过；fail 的 `.stderr` **只断言本库经 `syn::Error` 产出的稳定文案**（旧 `$`/`#()` 迁移报错、`widget`/`stateful` 参数错误、`#[component]` 非法参名），不绑定 rustc 类型错误。trybuild UI 测试须放 `ratatui-kit` crate（展开的 `::ratatui_kit::` 路径需运行时 crate 在场）。

- **组件渲染测试**（`src/render/harness.rs`，`#[cfg(test)]`）：`render_to_buffer(el, w, h)` 单次离屏渲染——no-op 终端跑 `update`（经对象安全的 `UpdaterTerminal` trait，无需真实 TTY）+ `ratatui::Terminal<TestBackend>` 跑 `draw` → 断言 `Buffer`。终端抽象对象安全化由 `render-test-harness` 落地：`ComponentUpdater` 持 `&mut dyn UpdaterTerminal`，`Tree` 暴露 `update_once`/`draw_root`；`Terminal<T>` 泛型保留（多后端），`UpdaterTerminal` 只暴露 update 阶段需要的 `insert_before`。**门控组件**（如 `router` 的 `RouterProvider`/`Outlet`）的渲染集成测试同样写在 `harness.rs`，以 `#[cfg(feature = "router")] mod router_tests` 门控并复用 `render_to_buffer`——内部用零状态 `#[component]` 测试页（渲染可辨识文本）+ `routes!` 搭路由表，断言 `index_path` 选中并渲染正确组件、嵌套 `Outlet` 消费剩余 path。

**正确做法**：改公开 API/宏后既跑 examples 冒烟,也跑 `--lib`/`--tests`;新增的纯逻辑（key/状态/路由匹配等）优先补 `#[cfg(test)]` 单测，宏的报错质量用 trybuild fail 用例锁住。

**相关文件**：`examples/`、`crates/ratatui-kit/tests/ui.rs`、各模块 `#[cfg(test)] mod tests`、`CLAUDE.md`

### 发布走 release.sh + tag 触发 CD

`release.sh [level] [exclude-crate ...]` 用 `cargo release` 逐 crate 升版本 + git-cliff 生成 CHANGELOG，最后 `git push origin main --tags`。tag 形如 `<crate>-v<version>`，由 `.github/workflows/CD.yaml`（`on: tags '*-v*'`）触发 `cargo publish crates/<crate>`。

**正确做法**：三个 crate（`ratatui-kit-macros` / `ratatui-kit` / `ratatui-kit-examples`）版本相互独立，按需用 `--exclude` 或交互式选择跳过不发布的 crate。`ratatui-kit-examples` 通常不发布到 crates.io。

**不要做**：手动 `cargo publish` 绕过脚本——会漏掉 CHANGELOG 生成与 tag 约定，导致 CD 的 release notes 抽取（按 CHANGELOG 段落 awk）拿到空内容。

**相关文件**：`release.sh`、`release.toml`、`.github/workflows/CD.yaml`

### 主库 README 是指向根 README 的符号链接（单一数据源）

仓库根 `README.md` 是唯一维护的文档；`crates/ratatui-kit/README.md` 是指向它的**符号链接**（`../../README.md`），`Cargo.toml` 用 `readme = "README.md"`。

**为什么不用 `readme = "../../README.md"`**：cargo 不支持包外 readme 路径——`cargo package` 会忽略它并只打包 crate 根内的 `README.md`（实测 warning：`readme '../../README.md' appears to be a path outside of the package`）。删掉子 README 后 crates.io 将拿不到任何 readme。符号链接则被 cargo 解引用，把根 README 的**内容**嵌入 `.crate` 包（实测为真实 `-rw-` 文件而非软链），crates.io / docs.rs 正常渲染且无 warning。

**正确做法**：改 README 只改根 `README.md`；新增需要 README 的可发布 crate，同样 `ln -s ../../README.md crates/<crate>/README.md` + `readme = "README.md"`。
**验证**：`cargo package -p ratatui-kit --allow-dirty --no-verify` 后 `tar tvzf target/package/ratatui-kit-*.crate | grep README` 应看到 `-rw-`（真实文件）。

**相关文件**：`README.md`、`crates/ratatui-kit/README.md`(symlink)、`crates/ratatui-kit/Cargo.toml`

### workspace 成员目录为 `crates/`，多处功能引用与之耦合

workspace 成员位于 `crates/ratatui-kit` 与 `crates/ratatui-kit-macros`（曾名 `packages/`，已改名）。改这个目录名要同步几处**功能性**引用，否则 CI/CD/hook 会静默失效：

- 根 `Cargo.toml` 的 `members` 与 examples 的 dev-dependency `path`
- `.github/workflows/CD.yaml` 的 `CRATE_PATH="crates/<prefix>"`（按 tag 前缀定位 crate 发布）
- `.claude/settings.json` 的 Stop hook 正则 `^(crates/.*/src/|examples/)`

文档类引用（`CLAUDE.md` / `AGENTS.md` / `dev-notes/` / docs 站）一并更新；`openspec/changes/**` 为历史快照，保持原样。

**相关文件**：`Cargo.toml`、`.github/workflows/CD.yaml`、`.claude/settings.json`
