## [0.7.1] - 2026-06-17

### 🐛 Bug Fixes

- *(macros)* Element! 子节点位接纳宏调用(element!/vec! 当 embed)
- *(components)* Widget() 接纳只实现按值 Widget 的部件(如 BigText)

### 💼 Other

- *(brand)* 由 favicon.svg 生成 GitHub 头像(icon-512)与社交预览(social-preview) PNG
- *(brand)* 头像改为满幅方形青底,去掉圆角/奶白外框(GitHub 自带圆角遮罩)
- *(brand)* Icon-512 改为 favicon.svg 直接栅格化(透明背景,不加处理)

### 📚 Documentation

- *(changelog)* Update for ratatui-kit-v0.7.0 [skip ci]
- *(home)* 英文首页对齐中文滚动落地页 + 修复 header 沉浸失效
- *(skill)* 安装命令补 --skill 参数(npx skills add yexiyue/ratatui-kit --skill ratatui-kit)+ frontmatter 去 Claude 限定
- *(skill)* 补两处 footgun(element! {} children 歧义 / widget() 按值边界)

### ⚙️ Miscellaneous Tasks

- Actions/checkout v4→v5(迁移到 Node 24,消除 Node 20 弃用告警)
## [0.7.0] - 2026-06-17

### 🚀 Features

- Element! 一等 if/for/match 控制流 + Text 文本节点 + adapter 按引用渲染
- *(router)* Routes! 宏支持像 element! 一样传 props
- *(core)* 重写事件系统为输入层 + 中央分发器,实现框架级输入互斥
- 重写文档示例与内置组件体系
- *(docs)* 首页重设计 + 品牌 logo/配色 + 依赖升级到 astro 6
- 新增 ratatui-kit AI agent skill 及配套文档,并修正过时文档

### 🐛 Bug Fixes

- *(atom)* Use_atom 跟随传入 atom 并在切换/卸载时退订 waker
- *(router)* 动态路由正则锚定开头 + 静态段转义，补全路由测试网
- *(core)* 实施 harden-audit-findings 审查修复(借用安全/反 panic/状态重构/hooks)
- *(ci)* GitHub Pages 部署 Node 升到 22(pnpm@latest 需 Node≥22.13,否则缺 node:sqlite 崩溃)
- *(ci)* Pnpm onlyBuiltDependencies 放行 esbuild/sharp 构建脚本(否则 CI 报 ERR_PNPM_IGNORED_BUILDS 退出 1)
- *(ci)* OnlyBuiltDependencies 迁到 pnpm-workspace.yaml(pnpm 11 不再读 package.json 的 pnpm 字段,改用 workspace 文件放行 esbuild/sharp)
- *(ci)* Pnpm-workspace.yaml 改用 allowBuilds(pnpm 11 移除 onlyBuiltDependencies)放行 esbuild/sharp 构建脚本

### 🚜 Refactor

- Element! DSL 去 sigil，$/#() 改为 widget()/stateful()/{ } 语法
- *(components)* Border/Positioned 用 from_props 单一构造源消除 new/update 重复
- *(runtime)* Drop component send sync bounds
- [**breaking**] 全局 store 重设计为 Atom（Jotai 式）+ 命名统一 + 运算符去重
- *(macros)* 抽出 ParsedElementHead，element codegen 单一真源（行为零变化）

### 📚 Documentation

- 新增 dev-workflow 项目知识库（dev-notes/knowledge）
- 纳入 AGENTS.md（Codex 项目指引，CLAUDE.md 的兄弟文件）
- *(openspec)* 提案 extract-parsed-element-head（ParsedElementHead 重构）
- 添加终端录制博客素材
- *(knowledge)* 记录 Pages 部署 Node22 + pnpm11 allowBuilds 坑

### ⚡ Performance

- *(router)* 路由匹配正则改为构造期一次性编译并缓存

### 🧪 Testing

- 全库测试网(一) 运行时单测 + 宏 trybuild UI 测试
- 全库测试网(二) 抽出 Route::match_path + router/store 单测
- *(render)* 终端抽象对象安全化 + 离屏渲染 harness + 组件渲染测试

### ⚙️ Miscellaneous Tasks

- .gitignore 放开 agent 配置，纳入 .claude/ CLAUDE.md openspec/
- Drop-send-sync 收尾 + gitignore .codex/
- *(openspec)* 归档 6 个已实现变更，spec deltas 落入 specs/
- *(openspec)* 归档 extract-parsed-element-head，spec delta 落入 specs/
- 重命名 workspace 目录 packages→crates 并重写 README
## [0.6.0] - 2026-06-10

### 🚀 Features

- [**breaking**] 迁移 ratatui 0.30 + 依赖统一升级

### 📚 Documentation

- Migrate docs

### ⚙️ Miscellaneous Tasks

- Release
## [0.5.9] - 2025-10-30

### 🐛 Bug Fixes

- 修复输入组件光标位置计算问题

### ⚙️ Miscellaneous Tasks

- Release
## [0.5.8] - 2025-10-26

### 🐛 Bug Fixes

- 修复ScrollView绘制时区域计算，确保内部内容正确显示

### ⚙️ Miscellaneous Tasks

- Release
## [0.5.7] - 2025-10-25

### 🚀 Features

- ScrollView支持边框

### ⚙️ Miscellaneous Tasks

- Update todo
- Release
## [0.5.6] - 2025-10-10

### 🚀 Features

- 添加use_on_drop hook

### ⚙️ Miscellaneous Tasks

- Release
## [0.5.5] - 2025-10-09

### 🚀 Features

- 添加TreeSelect组件
- 添加RouteState, 优化创建路由状态复杂问题

### 🐛 Bug Fixes

- 修复TreeSelect组件边框问题

### 💼 Other

- Todo

### ⚙️ Miscellaneous Tasks

- Release
## [0.5.4] - 2025-10-03

### 🚀 Features

- Handle支持返回参数

### ⚙️ Miscellaneous Tasks

- Release
## [0.5.3] - 2025-10-01

### 🚀 Features

- 添加固定位置组件，优化text组件参数
- 添加input 组件

### ⚙️ Miscellaneous Tasks

- Release
## [0.5.2] - 2025-09-30

### 🚀 Features

- 添加居中布局组件

### 🐛 Bug Fixes

- 修复panic信息被覆盖问题
- 修复transparent 布局不生效bug

### ⚙️ Miscellaneous Tasks

- Release
## [0.5.1] - 2025-09-28

### 🚀 Features

- 移除unstable-widget-ref

### 🎨 Styling

- Clippy
- Fmt

### ⚙️ Miscellaneous Tasks

- Release
## [0.5.0] - 2025-09-28

### 🚀 Features

- 支持StatefulWidget

### 🎨 Styling

- Clippy

### ⚙️ Miscellaneous Tasks

- Release
## [0.4.2] - 2025-09-26

### 🐛 Bug Fixes

- 修复ScrollView无限滚动问题 close #2
- Clippy

### ⚙️ Miscellaneous Tasks

- Release
## [0.4.1] - 2025-09-26

### 🚀 Features

- Add use terminal size

### 🐛 Bug Fixes

- Badges

### 📚 Documentation

- Add badges

### ⚙️ Miscellaneous Tasks

- Release
## [0.4.0] - 2025-07-09

### 📚 Documentation

- 完善文档
- 更新文档

### 🎨 Styling

- Clippy
- Fmt

### ⚙️ Miscellaneous Tasks

- Release
## [0.3.5] - 2025-07-05

### 🚀 Features

- 优化textarea
- Insert_before添加render_before方法
- 优化example

### 🐛 Bug Fixes

- 修复adapter

### ⚙️ Miscellaneous Tasks

- Release
## [0.3.4] - 2025-07-04

### 🚀 Features

- Add hook use_insert_before

### 🎨 Styling

- Clippy

### ⚙️ Miscellaneous Tasks

- Release
## [0.3.3] - 2025-07-03

### 🚀 Features

- Add with_layout_style util

### 🎨 Styling

- Clippy

### ⚙️ Miscellaneous Tasks

- Release
## [0.3.2] - 2025-06-27

### 🐛 Bug Fixes

- 优化Store宏

### 🎨 Styling

- Element

### ⚙️ Miscellaneous Tasks

- Release
## [0.3.1] - 2025-06-21

### 🚀 Features

- Add global store

### ⚙️ Miscellaneous Tasks

- Release
## [0.3.0] - 2025-06-19

### 🚀 Features

- Add ContextProvider
- Add router
- Add use_navigate
- Support router history and router params
- Add routes macro

### 🐛 Bug Fixes

- 优化element宏
- 优化Outlet组件,修复路由路径bug

### 🎨 Styling

- Clippy

### ⚙️ Miscellaneous Tasks

- Release
## [0.2.3] - 2025-06-17

### 🚀 Features

- Add ScrollView

### ⚙️ Miscellaneous Tasks

- Release
## [0.2.2] - 2025-06-12

### 🚀 Features

- Add hook useMemo
- Add hook useEffect

### 🐛 Bug Fixes

- 修复ci自动发布release body为空的bug

### ⚙️ Miscellaneous Tasks

- Release
## [0.2.1] - 2025-06-11

### 💼 Other

- 自动发布release
- 修复CD触发逻辑

### 🎨 Styling

- Clippy

### 🧪 Testing

- CI/CD

### ⚙️ Miscellaneous Tasks

- Release
- Release
## [0.2.0] - 2025-06-11

### 🚀 Features

- Add 'TextArea' component

### 💼 Other

- Element支持rest参数

### ⚙️ Miscellaneous Tasks

- Release
## [0.1.0] - 2025-06-09

### 🚀 Features

- Render
- Element extension
- Component macro
- Support widget
- Add border component
- Add modal
- UseEvents
- Add readme
- Use cargo release

### 💼 Other

- InstantiatedComponent

### ⚙️ Miscellaneous Tasks

- Release
