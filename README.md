# Ratatui Kit

[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/yexiyue/ratatui-kit)![Crates.io Version](https://img.shields.io/crates/v/ratatui-kit)![Crates.io Total Downloads](https://img.shields.io/crates/d/ratatui-kit)![docs.rs](https://img.shields.io/docsrs/ratatui-kit)[![Static Badge](https://img.shields.io/badge/%E5%AE%98%E6%96%B9%E7%BD%91%E7%AB%99-blue)](https://yexiyue.github.io/ratatui-kit-website/)

Ratatui Kit 是一个基于 [ratatui](https://github.com/ratatui-org/ratatui) 的 Rust 终端 UI 组件化开发框架，灵感来源于 React 生态，专注于高效、可组合、易维护的终端 UI 构建体验。

## 特性

- **声明式组件开发**：支持类似 React 的组件、props、hooks、context、路由等机制
- **丰富的 Hooks 支持**：内置 use_state、use_future、use_events、use_context、use_memo、use_effect 等常用 hooks
- **终端路由系统**：支持嵌套路由、动态参数、路由跳转，API 类似 React Router
- **全局状态管理**：支持全局 store 派生与注入，便于跨组件状态共享
- **异步渲染**：天然支持 tokio 异步生态，适合实时终端应用
- **与 ratatui 深度集成**：可无缝调用 ratatui 的全部能力
- **易扩展**：支持自定义组件、宏和 hooks

## 安装

在你的 Rust 项目中添加依赖：

```bash
cargo add ratatui-kit
```

如需使用路由、全局状态等高级功能，可在 `Cargo.toml` 中启用对应特性：

```toml
ratatui-kit = { version = "*", features = ["router", "store"] }
```

## 快速上手

参考[快速入门](https://yexiyue.github.io/ratatui-kit-website/docs/quick-start/)文档，体验从 0 到 1 的完整开发流程。

## 文档与示例

- [组件指南](https://yexiyue.github.io/ratatui-kit-website/docs/component/)
- [Hooks 指南](https://yexiyue.github.io/ratatui-kit-website/docs/hooks/)
- [全局状态](https://yexiyue.github.io/ratatui-kit-website/docs/global-state/)
- [路由](https://yexiyue.github.io/ratatui-kit-website/docs/router/)
- [更多示例](https://yexiyue.github.io/ratatui-kit-website/example/)

## 贡献与交流

欢迎 issue 和 PR！如有建议或 bug，请提交到 [GitHub Issues](https://github.com/yourname/ratatui-kit/issues)。

## License

MIT
