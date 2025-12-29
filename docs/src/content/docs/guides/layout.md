---
title: "布局"
sidebar:
    order: 3
---

Ratatui Kit 提供了统一且灵活的终端 UI 布局能力，其核心在于 `LayoutStyle`。通过合理配置 `LayoutStyle`，可以高效描述和控制组件的排列方式、对齐、间距等常见布局需求。

## LayoutStyle 概述

`LayoutStyle` 是 Ratatui Kit 各类支持布局组件（如 `View`、`Border`、`ScrollView` 等）通用的布局描述结构。开发者可在 Props 中直接设置相关字段，底层布局引擎将自动完成排布。

### 主要属性说明

- **主轴方向（`flex_direction`）**：决定子元素排列方向。
  - `Direction::Horizontal`：水平排列（默认）
  - `Direction::Vertical`：垂直排列
- **主轴对齐（`justify_content`）**：控制子元素在主轴上的分布。
  - `Flex::Start`：起始对齐
  - `Flex::End`：末尾对齐
  - `Flex::Center`：居中
  - `Flex::SpaceBetween`：两端对齐，元素间均分
  - `Flex::SpaceAround`：元素周围均分，首尾有间隙
  - `Flex::SpaceEvenly`：所有间隔均等
- **间距（`gap`）**：设置子元素间的空隙，单位为字符宽/高。
- **外边距（`margin`）**：控制组件与外部的距离。
- **偏移量（`offset`）**：微调组件位置。
- **尺寸（`width` / `height`）**：支持固定值、百分比等方式约束宽高。

## 支持 LayoutStyle 的组件

只要组件的 Props 结构体包含上述布局字段，即具备完整布局能力。常见支持组件包括：

- `Border`
- `View`
- `ScrollView`

## 布局示例

以下为垂直居中布局的典型用法，摘自 `examples/counter.rs`：

```rust
element!(
    Border(
        flex_direction: Direction::Vertical,
        justify_content: Flex::Center,
    ) {
        View(height: Constraint::Length(1)) {
            $Line::styled(
                format!("Counter: {state}"),
                Style::default().green().bold(),
            )
            .centered()
            .bold()
        }
    }
)
```

在该示例中，`Border` 和 `View` 的 Props 直接包含了布局相关字段，布局属性通过 Props 传递，底层由布局引擎统一处理。

通过合理使用 `LayoutStyle`，你可以灵活实现各种终端 UI 布局，满足不同场景的需求。建议优先熟悉各属性的作用，结合实际组件灵活配置。
