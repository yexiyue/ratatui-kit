# Ratatui Kit

[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/yexiyue/ratatui-kit) ![Crates.io Version](https://img.shields.io/crates/v/ratatui-kit) ![Crates.io Total Downloads](https://img.shields.io/crates/d/ratatui-kit) ![docs.rs](https://img.shields.io/docsrs/ratatui-kit) [![Static Badge](https://img.shields.io/badge/%E5%AE%98%E6%96%B9%E7%BD%91%E7%AB%99-blue)](https://yexiyue.github.io/ratatui-kit/)

Ratatui Kit 是一个基于 [ratatui](https://github.com/ratatui-org/ratatui) 的 Rust 终端 UI 组件化开发框架，灵感来源于 React 生态，专注于高效、可组合、易维护的终端 UI 构建体验。

## 特性

- **声明式组件开发**：支持类似 React 的组件、props、hooks、context、路由等机制
- **丰富的 Hooks 支持**：内置 `use_state`、`use_future`、`use_event_handler`、`use_context`、`use_memo`、`use_effect` 等常用 hooks
- **终端路由系统**：支持嵌套路由、动态参数、路由跳转，API 类似 React Router
- **全局状态管理**：支持 `Atom` / `use_atom`，便于跨组件状态共享
- **输入层互斥**：弹窗、搜索框、编辑层可以独占键盘事件，避免背景组件抢输入
- **内置组件**：提供输入框、弹窗、选择器、树形选择、虚拟列表、长文本换行和滚动容器
- **异步渲染**：天然支持 tokio 异步生态，适合实时终端应用
- **与 ratatui 深度集成**：可无缝调用 ratatui 的全部能力
- **易扩展**：支持自定义组件、宏和 hooks

## 安装

在你的 Rust 项目中添加依赖：

```bash
cargo add ratatui-kit
```

默认 feature 为空。学习或开发完整应用时可以先启用 `full`：

```toml
ratatui-kit = { version = "0.6.0", features = ["full"] }
```

也可以按需启用 `router`、`atom`、`input`、`tree`、`virtual-list`。

## 快速上手

参考 [快速开始](https://yexiyue.github.io/ratatui-kit/start/quick-start/) 文档，体验从 0 到 1 的完整开发流程。

## 文档与示例

- [安装与功能门控](https://yexiyue.github.io/ratatui-kit/start/installation/)
- [渐进式学习路径](https://yexiyue.github.io/ratatui-kit/start/)
- [Hooks 参考](https://yexiyue.github.io/ratatui-kit/core/hooks/)
- [状态模型](https://yexiyue.github.io/ratatui-kit/core/state/)
- [路由](https://yexiyue.github.io/ratatui-kit/core/routing/)
- [内置组件](https://yexiyue.github.io/ratatui-kit/components/)
- [示例路线图](https://yexiyue.github.io/ratatui-kit/examples/)

## 贡献与交流

欢迎 issue 和 PR！如有建议或 bug，请提交到 [GitHub Issues](https://github.com/yexiyue/ratatui-kit/issues)。

## License

MIT
