---
title: "介绍"
sidebar:
    order: 1
---

## 什么是 Ratatui Kit？

Ratatui Kit 是一个基于 [ratatui](https://github.com/ratatui-org/ratatui) 的 Rust 终端 UI 组件化开发框架。它借鉴了 React 的组件模型，将组件、props、hooks 等理念引入终端 UI，让开发变得高效、可组合、易维护。

## Ratatui Kit 如何工作？

Ratatui Kit 通过宏和 trait 实现组件、props、hooks 等机制，开发者可以像写前端一样编写终端 UI。它天然支持 tokio 异步生态，支持自定义组件，并与 ratatui 完美兼容，能够直接调用 ratatui 的全部能力。

## 为什么要用 Ratatui Kit？

Ratatui Kit 适合你，如果你：

- 希望像写 React 一样开发终端 UI，享受组件化、响应式开发体验
- 想要更快地构建复杂、可维护的终端应用
- 需要异步渲染和事件处理能力
- 希望在 Rust 生态下拥有现代化的终端 UI 解决方案
- 需要高度可扩展、可自定义的 UI 框架

Ratatui Kit 不适合你，如果你：

- 喜欢重复造轮子，手写所有终端 UI 逻辑
- 不需要组件化、响应式开发范式
- 只做极其简单的终端输出

## Ratatui Kit 与 ratatui 有什么区别？

ratatui 是一个专注于终端渲染和基础组件的底层库，提供了强大的终端 UI 能力。而 Ratatui Kit 在此基础上，进一步抽象出高层的组件模型和开发范式，让 UI 结构更清晰、代码更易维护。你既可以直接调用 ratatui 的底层能力，也可以用 Ratatui Kit 的组件体系高效搭建应用。

## 主要特性

- 组件化开发，支持 props、hooks、生命周期等机制
- 支持 use_state、use_future、use_events 等常用 hooks
- 异步渲染，适配 tokio 生态
- 易于扩展和自定义
- 适合构建复杂、可维护的终端 UI 应用

## 适用场景

- 快速开发终端 UI 工具、仪表盘、交互式 CLI 应用
- 希望在 Rust 生态下体验类似 React 的开发范式
- 需要异步、响应式的终端界面开发能力

## Ratatui Kit 是免费开源的吗？

是的，Ratatui Kit 遵循 MIT 协议，完全开源免费。你可以在任何项目中自由使用，无论是商业还是非商业用途。

## 如何支持 Ratatui Kit？

你可以通过在项目中使用 Ratatui Kit、向朋友和同事推荐、参与贡献代码或文档等方式支持本项目。

如需快速上手和更多示例，请参考项目的 [README](./packages/ratatui-kit/README.md) 和 [examples/](./examples) 目录。
