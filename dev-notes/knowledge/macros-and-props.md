# Macros & Props（过程宏与 props 类型擦除）

## 概览

本主题覆盖 `ratatui-kit-macros` 的五个过程宏（`element!` / `#[component]` / `#[derive(Props)]` / `routes!` / `#[derive(Store)]` / `#[with_layout_style]`）的非显然约定，以及 props 类型擦除（`AnyProps`）的 unsafe 不变量、`extern crate self` 技巧、ratatui 0.30 的 `SendBlock` 包装。改宏库、写新组件 props、或排查「宏展开路径/类型擦除」相关报错前先读本文件。

## 过程宏约定

### `#[component]`：参数只认 `props` / `hooks`，函数体被搬进 update

`#[component]` 把 `fn Foo(hooks, props) -> impl Into<AnyElement>` 重写为单元结构体 + `Component` 实现：函数体搬进 `implementation`，在 `update` 中执行，返回的 element 作为唯一子节点。参数名**仅识别 `props` / `hooks`**（及 `_` 前缀变体 `_props` / `_hooks`）。

**正确做法**：函数组件签名严格用 `props`/`hooks` 命名；不需要的参数用 `_props`/`_hooks`。记住它生成透明布局组件（见 `runtime-architecture.md`），布局属性写在返回的根元素上。

**不要做**：给参数起别的名字（如 `p`/`h`）——宏识别不到，展开报错。

**相关文件**：`packages/ratatui-kit-macros/src/component.rs`

### `element!`：一等控制流、`#(expr)` 内嵌、`$expr` 桥接原生 widget

- `Comp(prop: val) { children }` 构造子树。
- **一等控制流（首选）**：子节点块内直接写 `if`/`if let`/`else if`/`for`/`match`，分支体即子节点。
  codegen 把分支包在 `if/for/match` 外、内部仍是 `extend_with_elements` 调用——故**各分支可返回不同元素类型，无需 `.into_any()` 统一类型**。`for`/`match` 分支体用 `{}` 包裹；`for` 循环里的元素须给 `key:`（如 `key: i`）保证列表项稳定身份。
- `#(expr)` 仍可内嵌任意返回 `Option` / `Vec` / `impl Iterator<Item = Element>` / `Element` 的 Rust 表达式（动态/复杂场景的逃生）。
- **`$expr` 前缀**：通过 adapter 桥接实现 `Widget` 的 ratatui 原生 widget；`$(expr, state)` 桥接 `StatefulWidget`。是「逃生舱」，文本场景优先用 `Text(text: ...)`（见下）。

**正确做法**：条件/列表渲染优先用一等控制流；纯文本优先 `Text(text: "..." 或 Line/Text)`；只有 StatefulWidget（`$(w, s)`）或确需直接塞原生 widget 时才用 `$`。

```rust
// 条件：各分支不同元素类型，无需 into_any
element!(View { if loading.get() { Loading() } else { Content(data: d) } })
// 列表：内联 for，免去宏外 .map().collect::<Vec<AnyElement>>() + #()
element!(View { for (i, x) in items.iter().enumerate() { Row(label: x, key: i) } })
```

**相关文件**：`packages/ratatui-kit-macros/src/element.rs`（`parse_children` / `ControlFlow` / `to_extend`）、`packages/ratatui-kit-macros/src/adapter.rs`、`packages/ratatui-kit/src/components/text.rs`（`Text` 组件、`TextParagraph: From<&str>/Line/Text`）

### adapter 按引用渲染（ratatui 0.30）：bound 用 `for<'a> &'a T: Widget`，非 `Clone`

`WidgetAdapter`/`StatefulWidgetAdapter` 的 `draw` 用 `render_widget(&self.inner, ..)` 按引用渲染，免去每帧 clone。ratatui 0.30 起所有内置 widget 都实现 `Widget for &T`（`WidgetRef` 仍是 unstable，不依赖它）。

**正确做法**：自定义 widget 想经 `$` 嵌入时，除 `Widget for T` 外最好也实现 `Widget for &T`（ratatui 官方推荐）；否则会因 adapter 的 `for<'a> &'a T: Widget` 约束编译失败（参考 `text.rs` 给 `&TextParagraph` 补的 impl）。`new`/`update` 仍需 `Clone` 从借用 props 拷入持久组件，无法省。

**相关文件**：`packages/ratatui-kit/src/components/adapter/widget.rs`、`stateful_widget.rs`

### `#[with_layout_style]`：组件获得布局能力的标准方式

给 Props 结构体注入布局字段（width/height/flex_direction…）并生成 `layout_style()`。这是让自定义组件支持布局属性的标准入口。

**正确做法**：新组件想支持 `element!` 里的布局属性，就给它的 Props 加 `#[with_layout_style]`，参考 `components/view.rs`。

**相关文件**：`packages/ratatui-kit-macros/src/with_layout_style.rs`、`packages/ratatui-kit/src/components/view.rs`

## Props 类型擦除与 unsafe 不变量

### `Option<T>` 字段已天然接收裸值，无需 `Some(...)`，不要再造 `OptionalProp` 轮子

`element!` 对**每个**字段值统一生成 `(expr).into()`（见 `element.rs` 的 `PropsItem::to_tokens`）。
配合 std 的 `impl<T> From<T> for Option<T>`（裸值包 `Some`）与反身 `From<Option<T>> for Option<T>`（直通），
一个普通 `Option<T>` 字段**本来就**同时接收：裸 `T`（自动 `Some`）、`Some(x)`、`None` 三种写法。

**正确做法**：可选字段直接声明 `Option<T>`，调用方按需写 `top_title: Line::from("提示")` 或 `top_title: Some(...)` 或 `top_title: None`，全部成立。

**不要做**：为了「省掉 `Some(...)`」去新造 `OptionalProp<T>` 包装类型——纯属多余且破坏性（改字段类型 + 破坏读取点）。曾验证：`(裸值).into()` / `(Some(x)).into()` / `(None).into()` 对 `Option<T>` 全部编译通过。

**相关文件**：`packages/ratatui-kit-macros/src/element.rs`（`PropsItem::to_tokens` 的 `.into()`）、`packages/ratatui-kit/src/components/border.rs`（`top_title`/`bottom_title: Option<Line>`，examples 已用裸值写法）

### Props 必须 Send + Sync，无 props 用 NoProps

props 必须实现 `Props`（`unsafe trait`，要求 `Send + Sync`），通过 `#[derive(Props)]` 生成。无 props 的组件用现成的 `NoProps`。

**正确做法**：新 Props 结构体的所有字段都得 `Send + Sync`，再 `#[derive(Props)]`。

**相关文件**：`packages/ratatui-kit/src/props.rs`

### ratatui 0.30：Block 不再 Send+Sync，props 持有 Block 必须用 SendBlock

ratatui 0.30 起 `Block` 内含 `Arc<dyn CellEffect>`（阴影效果的类型擦除句柄）而**不再 `Send + Sync`**。但 Props/Component 都要求 `Send + Sync`（组件 `wait()` 经 `BoxFuture`(Send) 轮询）。因此 props 里要承载 `Block` 时，用 `SendBlock`（`Option<Block<'static>>` 的 `Send + Sync` 包装）而非裸 `Block`。

**正确做法**：props 边框字段写 `block: SendBlock`。`SendBlock` 实现了 `Deref<Target = Option<Block>>` + `From<Block>` / `From<Option<Block>>`，配合 `element!` 自动 `.into()`，书写与原 `Option<Block<'static>>` 完全一致（如 `block: Block::bordered()...`），`.is_some()`/`.as_ref()` 照常可用。

**不要做**：在 props/组件字段里直接放裸 `Block<'static>`——会因不满足 `Send + Sync` 编译失败。

**相关文件**：`packages/ratatui-kit/src/components/send_block.rs`、`packages/ratatui-kit/src/props.rs`

### AnyProps 的 unsafe downcast 依赖协调阶段已校验 TypeId

`AnyProps` 用类型擦除裸指针 + 手动 drop 在「借用」和「拥有」两种 props 间转换。`downcast_*_unchecked` 是 unsafe，**正确性依赖协调阶段已经校验过 `TypeId` 匹配**（同 key 同 TypeId 才复用）。

**正确做法**：改 `props.rs` 或协调逻辑（`render/updater.rs`）时，务必保持「downcast 前 TypeId 已校验」这一不变量。新增 props 转换路径要复用现有的 borrow/own + 手动 drop 模式，别自行 `transmute`。

**相关文件**：`packages/ratatui-kit/src/props.rs`、`packages/ratatui-kit/src/render/updater.rs`

## 库内自引用

### `extern crate self as ratatui_kit` 让库内也能用本库宏

`lib.rs` 末尾的 `extern crate self as ratatui_kit;` 使库内代码也能用本库的过程宏——宏展开生成 `::ratatui_kit::...` 绝对路径，没有这行别名，库内（如 examples 之外的内部组件）用 `element!` 会找不到 crate。`prelude` 汇出常用项，示例统一 `use ratatui_kit::prelude::*;`。

**正确做法**：新增需要在库 crate 内部使用 `element!`/`#[component]` 的模块时，依赖这个 self 别名即可，无需额外引入。

**相关文件**：`packages/ratatui-kit/src/lib.rs`
