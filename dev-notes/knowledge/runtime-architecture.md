# Runtime Architecture（声明 → 实例 → 渲染循环）

## 概览

本主题覆盖 ratatui-kit 运行时的核心机制：`Element`（声明）vs `Component`（实例）的二分、协调（reconciliation）的 key 复用语义、`render_loop` 的「状态变更经 Waker 唤醒」响应式模型、以及布局/透明布局。**改 `render/`、`component/`、`element/` 任一模块前先读本文件**——这些不变量一旦破坏，整棵树的状态保持或重绘触发会静默失效。

## 声明 vs 实例

### Element 是声明，InstantiatedComponent 才是状态住所

- `Element<T>` = `{ key, props }`，**每帧重建**的轻量声明，`element!` 宏产出，`AnyElement` 是类型擦除版。
- `InstantiatedComponent` 是**持久化树节点**，持有 component 实例、hooks 列表、子 `Components`、`LayoutStyle`——状态（hooks、use_state）真正存活在这里，跨帧保留。

**正确做法**：理解「Element 廉价、可随意重建；Instantiated 昂贵、靠协调复用」。给组件的副作用/状态都挂在 hooks 上，而不是塞进 props 或 element。

**相关文件**：`packages/ratatui-kit/src/element/mod.rs`、`packages/ratatui-kit/src/component/instantiated_component.rs`

### 协调按 `ElementKey + TypeId` 复用——key 决定状态去留

`ComponentUpdater::update_children` 按 `ElementKey` + 组件 `TypeId` 匹配上一帧节点：命中则复用（保留 hooks/状态），否则新建。这就是「同一 key 同一类型 → 状态保留」的来源，等同 React 的 key diff。

**正确做法**：列表渲染时给每个子元素稳定且唯一的 `key`，否则增删项会错位复用别人的状态。换组件类型（即使 key 相同）会强制重建。

**不要做**：用数组下标当 key 渲染会增删的列表——会导致状态串台。

**相关文件**：`packages/ratatui-kit/src/render/updater.rs`、`packages/ratatui-kit/src/element/key.rs`

## 渲染循环与响应式

### UI 不是命令式重绘，而是状态写入 → Waker 唤醒 → 重渲染

`render/tree.rs` 的 `render_loop` 骨架：

```text
loop { render(); if should_exit || ctrl_c break; select(component.wait(), terminal.wait()).await }
```

`render()` 先自顶向下 `update`（跑组件函数体、跑 hooks、协调子树），再 `terminal.draw` 自顶向下 `draw`。然后 `select` 在「组件树有变化」与「终端有事件」之间阻塞，任一就绪即重渲染。

「组件树有变化」由 `poll_change` 聚合：组件 / 子节点 / hooks 三路任一 `Ready` 即唤醒。响应式状态（`use_state` 的 `State<T>`、全局 `AtomState<T>`，均基于 `generational-box`）在写入时唤醒存好的 `Waker`，打破 `select` 阻塞触发下一帧。

**正确做法**：要让 UI 响应某个变化，必须让它经过一个会唤醒 Waker 的通道——`State`/`AtomState` 写入、`use_future` 完成、终端事件。自定义 Hook 若持有需驱动重绘的状态，要实现 `poll_change` 并在变更时唤醒 waker（参见 `hooks-and-state.md`）。

**不要做**：在组件外用普通变量/`static mut` 存 UI 状态再期望它自动重绘——没有 Waker 唤醒，`select` 不会醒，画面卡住直到下一个无关事件。

**相关文件**：`packages/ratatui-kit/src/render/tree.rs`、`packages/ratatui-kit/src/component/mod.rs`（`poll_change`）

## 布局

### LayoutStyle 直接映射 ratatui Layout/Constraint

`LayoutStyle` = `flex_direction / justify_content / gap / margin / offset / width / height`，直译为 ratatui 的 `Layout`/`Constraint`。`Component::calc_children_areas` 默认按 flex 切分子区域，可重写实现自定义布局。

**正确做法**：写需要非 flex 排布的组件（如 `ScrollView`、`Modal`）时重写 `calc_children_areas`，参考现有两者的实现。组件想获得布局字段，用 `#[with_layout_style]` 给 Props 注入（见 `macros-and-props.md`）。

**相关文件**：`packages/ratatui-kit/src/render/layout_style.rs`、`packages/ratatui-kit/src/components/scroll_view/mod.rs`、`packages/ratatui-kit/src/components/modal.rs`

### 透明布局：函数组件是「透传包装器」，布局属性写在子根元素上

`#[component]` 宏生成的函数组件会调用 `set_transparent_layout(true)`，使其**不占独立布局节点**、直接继承首个子组件的 `LayoutStyle`。

**正确做法**：给一个 `#[component]` 函数组件设 width/height/flex 等布局属性时，要把它们写在该函数 **返回的根元素** 上，而不是期望在父级 `element!` 里给这个函数组件传布局 prop 生效。

**不要做**：在 `element!` 里给一个函数组件包装器直接挂布局属性并指望它形成独立布局区——它是透明的，属性会被忽略/穿透。

**相关文件**：`packages/ratatui-kit-macros/src/component.rs`（`set_transparent_layout`）、`packages/ratatui-kit/src/components/view.rs`
