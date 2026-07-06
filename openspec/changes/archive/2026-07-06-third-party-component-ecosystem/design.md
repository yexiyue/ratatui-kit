## Context

当前所有组件只能通过「合并进主仓库」进入生态,导致主库 `ratatui-kit` 无限膨胀,且每次 ratatui / 框架 breaking 都要连带维护大批内置组件。框架仍处 `0.x`、刚经历「三次重写」,动荡剧烈。

已用一个「外部作者视角」的 probe crate 实证:框架公共 API 对第三方组件作者**已基本就绪**(手动 `impl Component`、`#[component]`、`element!`、`#[derive(Props)]`、`#[with_layout_style]`、`use_state`、自定义 `Hook` + `use_hook` 全部可用,架构无中央注册表),**唯一阻塞是 `#[with_layout_style]` 宏生成裸 `ratatui::layout::...` 路径**——外部 crate 作用域无 `ratatui` 名字即编译失败。

约束(来自项目知识库):单线程渲染、框架级已移除 `Send + Sync` 强制、宏展开须用绝对 `::ratatui_kit::` 路径、feature 门控「改了模块没开 feature 就编译不到」、「编译即基线(example + doctest)」、发布走「打 tag → CI publish + git-cliff」。

一次针对两个 PR 的多 agent 深度 review 佐证了本设计的几个判断:table(#11)完全满足 issue #10 且 CI 全绿;markdown(#12)携带 3 个实证 confirmed 的 correctness bug + 红 CI;且 review 独立发现官方 `Divider` 组件自己踩了「透明布局陷阱」(`#[with_layout_style]` 属性被静默忽略)——印证组件作者规范必须内建正确示范。

## Goals / Non-Goals

**Goals:**
- 解锁外部 crate 定义组件(修 `with_layout_style` 宏 hygiene)。
- 确立并文档化「扩展 API 稳定面」契约,让第三方 crate 有稳定依赖面。
- 产出组件作者规范 + `cargo-generate` 模板 + 命名/发布/发现约定。
- 落地试点:table 入核心、markdown 迁出为独立 crate。

**Non-Goals:**
- 不重构现有组件的既有 API。
- 不引入中央组件注册表 / 运行时插件动态加载(Rust 静态链接,组件即类型,无需)。
- 本阶段不收窄 / 删除任何现有 `pub` 项(避免再叠加 breaking);稳定面仅文档化 + `#[doc(hidden)]` 标注。
- 不建设 registry 后端服务(用 crates.io keyword + awesome-list 即足够)。

## Decisions

- **D1 宏路径修复用绝对 re-export 路径**:`with_layout_style` 注入字段类型改为 `::ratatui_kit::ratatui::layout::*`(经 lib.rs `pub use ratatui` 转发),而非要求外部 crate 自行 `use ratatui;`。理由:外部只依赖 `ratatui-kit`,避免 ratatui 版本双开;与同文件 `layout_style()` 方法体已有的 `::ratatui_kit::layout_style::` 风格一致。备选(要求外部依赖 ratatui)被否:强加依赖 + 版本地狱。

- **D2 三层结构 + 官方 contrib monorepo**:核心 / `ratatui-kit-contrib`(官方扩展,各自独立 crate)/ 社区独立 crate。理由:主库瘦身(直接解决核心痛点),官方扩展集中便于统一 CI 与版本管理,社区去中心化零维护成本。备选被否:纯 monorepo 多 crate(代码仍在主仓库,未解决膨胀);纯去中心化(官方质量与发现失控)。

- **D3 命名统一 `ratatui-kit-<name>` + keyword `ratatui-kit`**:理由:生态识别度(类 `bevy_`)、crates.io 搜索友好。风险 name squatting → 官方提前占用核心扩展名。备选被否:社区独立短前缀(与主品牌脱钩,新人难关联);scoped 包(Rust crates.io 无 scope)。

- **D4 稳定面本阶段只文档化 + `#[doc(hidden)]`,不删 `pub` 项**:理由:`0.x` 已足够动荡,先立契约、避免再加 breaking;真正收窄 API 留后续独立 change 并标 BREAKING。

- **D5 table 入核心 / markdown 迁出,且迁移前置修 bug**:判据「运行时 / 通用基础 → 核心;应用层重组件(依赖 pulldown-cmark / 语法高亮)→ 外置」。table 直接回应 issue #10 且 CI 全绿,作一等基础组件。markdown 迁出为 `ratatui-kit-markdown`,但 review 已实证其有 3 个 confirmed correctness bug(连续段落合并成一行、嵌套列表丢 bullet、标题样式泄漏)+ 红 CI + openspec 文档与实现不符——**这些 MUST 在迁移中一并修复并加回归测试**,否则等于把已知损坏的组件搬进生态。

- **D6 发现用 crates.io keyword + `awesome-ratatui-kit`,不建 registry 服务**:理由:零维护、Rust 生态惯例。

- **D7(待定) 宏 `crate = "..."` 逃生舱**:是否本阶段做。倾向:P0 可选——markdown 试点不 rename 依赖故非阻塞;但为长期健壮(防未来改名 / 使用方 rename)建议尽早补,归入 `extension-api-surface` 的宏 hygiene 延伸。

## Risks / Trade-offs

- **稳定面在 `0.x` 仍会 breaking** → 明确「`0.x` 的 semver:minor = breaking、patch = 新增」,外部用版本区间约束 + CHANGELOG 记录;把 probe 转正为「外部 crate 可编译」的持续验证(trybuild pass 用例或 contrib 里的最小 crate)。
- **name squatting** → 官方提前注册核心扩展名(`ratatui-kit-table` 等)。
- **contrib monorepo 版本协调成本** → 各 crate 独立版本 + 独立 tag 前缀,复用现有 CD 的按 tag 前缀定位 crate 机制。
- **宏路径修复回归** → trybuild `pass`/`fail` + 四件套 `--all-features` + 新增外部编译验证。
- **markdown 迁移暴露更多缺失扩展 API** → 视为预期收益(以战代练),缺什么补什么并回填稳定面清单。
- **迁移带入未修 bug** → 把 review 实证的 3 个 confirmed bug 作为迁移**验收项**,不修不迁。

## Migration Plan

1. **P0 先落地**(修宏 + 稳定面文档),对现有用户零影响(库内 `ratatui` 仍可达)。
2. **PR 合并顺序(review 已给专业结论,二者 git-stack 不可独立合)**:先合 #11(CI 绿、零验证存活 findings、单独即满足 issue #10);采纳 #12 对 `table/layout.rs` 的一行改动(Outer 模式 `column_count+1` 比 `2` 更正确,markdown 用 Grid 不受影响);#11 落地后 #12 rebase onto main,重复 table commit 自动 drop。
3. **markdown 迁移**:在 `ratatui-kit-contrib` 建 `ratatui-kit-markdown`,搬入组件源码,依赖改为 crates.io `ratatui-kit` 版本区间;**先修 review 的 3 个 confirmed bug + 红 CI(clippy clone-on-Copy、rustfmt)+ openspec 文档对齐**,再随模板/规范一起发布;#12 PR 转为「不合入主库」并致谢作者。
4. **回滚**:P0 宏修复若出问题,revert 单文件即可;文档 / 新仓库均为增量,无回滚风险。

## Open Questions

- 宏 `crate = "..."` 逃生舱是否纳入本阶段 P0?
- `ratatui-kit-contrib` 用单一统一版本还是各 crate 独立版本?(倾向独立)
- 核心该保留哪些一等组件、哪些下沉社区?(本 change 只处理 table 入核、markdown 出;其余组件盘点另议)
- PR #12 作者(KonghaYao)是否愿意以独立 crate 形式维护 markdown,还是由官方接管进 contrib?
- `table/layout.rs` Outer 模式 `render_row_line` 与 `render_border_line` 的深层不一致是否顺带修(review 标为可选 follow-up)?
