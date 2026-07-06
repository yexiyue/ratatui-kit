# Macros & Props（过程宏与 props 类型擦除）

## 概览

本主题覆盖 `ratatui-kit-macros` 的过程宏（`element!` / `#[component]` / `#[derive(Props)]` / `routes!` / `#[with_layout_style]`）的非显然约定，以及 props 类型擦除（`AnyProps`）的 unsafe 不变量、`extern crate self` 技巧、ratatui 0.30 后 `Block` props 的处理方式。改宏库、写新组件 props、或排查「宏展开路径/类型擦除」相关报错前先读本文件。

## 过程宏约定

### `#[component]`：参数只认 `props` / `hooks`，函数体被搬进 update

`#[component]` 把 `fn Foo(hooks, props) -> impl Into<AnyElement>` 重写为单元结构体 + `Component` 实现：函数体搬进 `implementation`，在 `update` 中执行，返回的 element 作为唯一子节点。参数名**仅识别 `props` / `hooks`**（及 `_` 前缀变体 `_props` / `_hooks`）。

**正确做法**：函数组件签名严格用 `props`/`hooks` 命名；不需要的参数用 `_props`/`_hooks`。记住它生成透明布局组件（见 `runtime-architecture.md`），布局属性写在返回的根元素上。

**不要做**：给参数起别的名字（如 `p`/`h`）——宏识别不到，展开报错。

**相关文件**：`crates/ratatui-kit-macros/src/component.rs`

### `element!`：一等控制流、`{ expr }` 内嵌、显式 adapter 桥接原生 widget

- `Comp(prop: val) { children }` 构造子树。
- **一等控制流（首选）**：子节点块内直接写 `if`/`if let`/`else if`/`for`/`match`，分支体即子节点。
  codegen 把分支包在 `if/for/match` 外、内部仍是 `extend_with_elements` 调用——故**各分支可返回不同元素类型，无需 `.into_any()` 统一类型**。`for`/`match` 分支体用 `{}` 包裹；`for` 循环里的元素须给 `key:`（如 `key: i`）保证列表项稳定身份。
- `{ expr }` 可内嵌任意返回 `Option` / `Vec` / `impl Iterator<Item = Element>` / `Element` 的 Rust 表达式（动态/复杂场景的逃生）。
- **adapter 显式节点**：`widget(expr)` 桥接实现 `Widget` 的 ratatui 原生 widget；`stateful(widget, state)` 桥接 `StatefulWidget`。是「逃生舱」，文本场景优先用 `Text(text: ...)`（见下）。

**正确做法**：条件/列表渲染优先用一等控制流；纯文本优先 `Text(text: "..." 或 Line/Text)`；只有 StatefulWidget（`stateful(w, s)`）或确需直接塞原生 widget 时才用 adapter。

```rust
// 条件：各分支不同元素类型，无需 into_any
element!(View { if loading.get() { Loading() } else { Content(data: d) } })
// 列表：内联 for，免去宏外 .map().collect::<Vec<AnyElement>>() + { expr }
element!(View { for (i, x) in items.iter().enumerate() { Row(label: x, key: i) } })
```

**相关文件**：`crates/ratatui-kit-macros/src/element.rs`（`parse_children` / `ControlFlow` / `to_extend`）、`crates/ratatui-kit-macros/src/adapter.rs`、`crates/ratatui-kit/src/components/text.rs`（`Text` 组件、`TextParagraph: From<&str>/Line/Text`）、`examples/core/control_flow.rs`

### `routes!`：右侧复用 `element!` 头部解析，可传 props

`routes!` 右侧 `"/path" => Component` 现支持像 `element!` 一样传 props：`"/path" => Component(prop: val)`。圆括号 `()` 传 props、花括号 `{}` 留给嵌套子路由，二者顺序固定且互斥。实现支点是 **`ParsedElementHead`**（只含 `ty` + `props`、**无 children 字段**）：它 `impl Parse` 只吃 `Ty` + 可选 `(props)`、不消费 `{}`，并以 `to_element_expr(children)` 作为 element codegen 的**单一真源**——`element!` 传实际子节点切片、`routes!` 传空切片。`ParsedElement` 则是 `{ head, children }` 的组合。no-props 路径与旧 `element!(#element)` 字节等价，`key`/`..rest`/`(expr).into()` 全部白拿。

**正确做法**：
- 静态配置走 props：`"/dash" => Dashboard(columns: 3) { "/panel" => Panel }`（标题、只读标志、列数等构造期常量）。
- 动态数据**不要**走 props：路径参数 `/:id` 用 `use_params`，导航载荷用 `use_route_state::<T>()` + `push_with_state`（对应 React 的 `useParams`/`useLoaderData`）。

**不要做 / 边界**：
- **`'static` 硬约束**：经 `routes!` 传入的 props 会被烘进 `AnyElement<'static>`，**不能借用栈上局部**。同样的 `Comp(prop: x)` 在组件内联使用时可持 `&borrow`，放进 `routes!` 却会被 `'static` 拒绝——报生命周期错时先查这条。
- **`key:` 被拒绝**：路由身份由 path 决定，`routes!` 在 `ParsedRoute::parse` 显式报错「路由组件不支持 key:」（经 `ParsedElementHead::key_span` 检测）。
- **「无 children」是类型强制、非注释约定**：`ParsedElementHead` 没有 children 字段，故「头部解析阶段消费 `{}`」结构上无法表达，`routes!` 持有的 head 也物理上无静态 children 可传——`{}` 必归子路由。回归护栏仍是 `router/mod.rs` 的 `routes_macro_accepts_props_with_children` 测试，勿删。
- **codegen 形态自洽**：`to_element_expr` 输出**带外层括号**的元素表达式 `({…})`，调用方（`router` 的 `.into_any()`、`element!` 嵌套 extend、方法链 `.fullscreen()`）直接用，不需补括号、不依赖内部是块——勿在 `router` 恢复手动加括号。
- **`key:` 字段查找单一真源**：`element.rs` 用 `PropsItem::as_key_field` 集中「是否 `key:` 字段」的判断（魔法串 `"key"` 只此一处），`to_element_expr` 的 key 构造/props 过滤与 `key_span` 都经它，勿再各自手写 `Member::Named("key")` 匹配。

**相关文件**：`crates/ratatui-kit-macros/src/router.rs`、`crates/ratatui-kit-macros/src/element.rs`（`ParsedElementHead`：`to_element_expr` / `key_span` / `PropsItem::as_key_field`；`ParsedElement` = head + children）、`crates/ratatui-kit/src/components/router/mod.rs`（routes! 传 props 测试）

### adapter 按引用渲染（ratatui 0.30）：bound 用 `for<'a> &'a T: Widget`，非 `Clone`

`WidgetAdapter`/`StatefulWidgetAdapter` 的 `draw` 用 `render_widget(&self.inner, ..)` 按引用渲染，免去每帧 clone。ratatui 0.30 起所有内置 widget 都实现 `Widget for &T`（`WidgetRef` 仍是 unstable，不依赖它）。

**正确做法**：自定义 widget 想经 `widget(...)` 嵌入时，除 `Widget for T` 外最好也实现 `Widget for &T`（ratatui 官方推荐）；否则会因 adapter 的 `for<'a> &'a T: Widget` 约束编译失败（参考 `text.rs` 给 `&TextParagraph` 补的 impl）。`new`/`update` 仍需 `Clone` 从借用 props 拷入持久组件，无法省。

**相关文件**：`crates/ratatui-kit/src/components/adapter/widget.rs`、`stateful_widget.rs`

### `#[with_layout_style]`：组件获得布局能力的标准方式

给 Props 结构体注入布局字段（width/height/flex_direction…）并生成 `layout_style()`。这是让自定义组件支持布局属性的标准入口。

**正确做法**：新组件想支持 `element!` 里的布局属性，就给它的 Props 加 `#[with_layout_style]`，参考 `components/view.rs`。

**相关文件**：`crates/ratatui-kit-macros/src/with_layout_style.rs`、`crates/ratatui-kit/src/components/view.rs`

## Props 类型擦除与 unsafe 不变量

### `Option<T>` 字段已天然接收裸值，无需 `Some(...)`，不要再造 `OptionalProp` 轮子

`element!` 对**每个**字段值统一生成 `(expr).into()`（见 `element.rs` 的 `PropsItem::to_tokens`）。
配合 std 的 `impl<T> From<T> for Option<T>`（裸值包 `Some`）与反身 `From<Option<T>> for Option<T>`（直通），
一个普通 `Option<T>` 字段**本来就**同时接收：裸 `T`（自动 `Some`）、`Some(x)`、`None` 三种写法。

**正确做法**：可选字段直接声明 `Option<T>`，调用方按需写 `top_title: Line::from("提示")` 或 `top_title: Some(...)` 或 `top_title: None`，全部成立。

**不要做**：为了「省掉 `Some(...)`」去新造 `OptionalProp<T>` 包装类型——纯属多余且破坏性（改字段类型 + 破坏读取点）。曾验证：`(裸值).into()` / `(Some(x)).into()` / `(None).into()` 对 `Option<T>` 全部编译通过。

**相关文件**：`crates/ratatui-kit-macros/src/element.rs`（`PropsItem::to_tokens` 的 `.into()`）、`crates/ratatui-kit/src/components/border.rs`（`top_title`/`bottom_title: Option<Line>`，examples 已用裸值写法）

### Props 是安全标记 trait，无 props 用 NoProps

props 必须实现 `Props`，通过 `#[derive(Props)]` 生成。PR 6 去掉了框架级 `Send + Sync` 要求后，`Props` 已从 `unsafe trait` 改为安全 trait；无 props 的组件用现成的 `NoProps`。

**正确做法**：新 Props 结构体直接 `#[derive(Props)]`。若组件需要布局属性，再叠加 `#[with_layout_style]`。

**相关文件**：`crates/ratatui-kit/src/props.rs`

### element! 通过 Default 补齐未传 props 字段

`element!` 构造 props 时会使用结构体更新语法 `..Default::default()` 补齐调用点没传的字段。因此任何要直接被 `element!` 构造的自定义 props 都需要实现 `Default`；字段全都有天然默认值时可 `#[derive(Default, Props)]`，否则手写 `impl Default`。

**正确做法**：
- 示例或业务自定义组件的 props 即使每次都会显式传满字段，也要给 `element!` 留出 `Default`。
- `Color`、回调、业务枚举等不方便 derive 的字段，用语义中性的默认值手写实现。

**相关文件**：`crates/ratatui-kit-macros/src/element.rs`、`examples/advanced/custom_provider.rs`

### ratatui 0.30：Block 可直接作为 props 字段

ratatui 0.30 起 `Block` 内含 `Arc<dyn CellEffect>`（阴影效果的类型擦除句柄）而**不再 `Send + Sync`**。PR 6 已去掉框架内部组件/Hook/Props 的 `Send + Sync` 强制要求，因此旧的 `SendBlock` 包装已删除，props 字段可以直接使用 `Option<Block<'static>>`。

**正确做法**：props 边框字段写 `block: Option<Block<'static>>`。配合 `element!` 自动 `.into()`，调用方仍可写裸 `Block`（如 `block: Block::bordered()...`）、`Some(block)` 或 `None`。

**不要做**：恢复 `SendBlock` 或为 `Block` 增加新的 Send/Sync 包装；当前运行时是单线程渲染，裸 `Block` 已是目标形态。

**相关文件**：`crates/ratatui-kit/src/components/scroll_view/mod.rs`、`crates/ratatui-kit/src/components/tree_select.rs`、`crates/ratatui-kit/src/props.rs`

### AnyProps 的 unsafe downcast 依赖协调阶段已校验 TypeId

`AnyProps` 用类型擦除裸指针 + 手动 drop 在「借用」和「拥有」两种 props 间转换。`downcast_*_unchecked` 是 unsafe，**正确性依赖协调阶段已经校验过 `TypeId` 匹配**（同 key 同 TypeId 才复用）。

**正确做法**：改 `props.rs` 或协调逻辑（`render/updater.rs`）时，务必保持「downcast 前 TypeId 已校验」这一不变量。新增 props 转换路径要复用现有的 borrow/own + 手动 drop 模式，别自行 `transmute`。

**相关文件**：`crates/ratatui-kit/src/props.rs`、`crates/ratatui-kit/src/render/updater.rs`

### AnyProps 借用句柄不得放大生命周期

`AnyProps::borrow(&mut self)` 必须返回 `AnyProps<'_>`，生命周期绑定到这次 `&mut self` 借用；`AnyElement::from(&mut AnyElement<'b>)` 同理只能产出借用期内的 `AnyElement<'_>`。这是防止从路由表、props 或临时 element 派生出的裸指针句柄逃逸。

**正确做法**：
- `AnyProps` 携带由 `ComponentHelper` 传入的 props `TypeId`，`downcast_ref_unchecked`/`downcast_mut_unchecked` 在 debug 下断言匹配。
- 遇到“借用 element 后返回”的生命周期错误时，改成在 `update_children` 期间立即消费，或把数据临时移出再放回；不要把返回类型改回更长生命周期。

**相关文件**：`crates/ratatui-kit/src/props.rs`、`crates/ratatui-kit/src/element/any_element.rs`、`crates/ratatui-kit/src/components/router/outlet.rs`

### with_layout_style 只支持具名字段结构体

`#[with_layout_style]` 会向 props 注入布局字段，因此只能用于具名字段结构体。元组结构体或单元结构体会产生稳定错误：``#[with_layout_style]` 只能用于具名字段结构体`。

**正确做法**：需要布局能力的 props 使用 `struct Props { ... }` 具名字段形态，再叠加 `#[with_layout_style]`。

**相关文件**：`crates/ratatui-kit-macros/src/with_layout_style.rs`、`crates/ratatui-kit/tests/ui/fail/with_layout_style_non_named.rs`

## 库内自引用

### `extern crate self as ratatui_kit` 让库内也能用本库宏

`lib.rs` 末尾的 `extern crate self as ratatui_kit;` 使库内代码也能用本库的过程宏——宏展开生成 `::ratatui_kit::...` 绝对路径，没有这行别名，库内（如 examples 之外的内部组件）用 `element!` 会找不到 crate。`prelude` 汇出常用项，示例统一 `use ratatui_kit::prelude::*;`。

**正确做法**：新增需要在库 crate 内部使用 `element!`/`#[component]` 的模块时，依赖这个 self 别名即可，无需额外引入。

**相关文件**：`crates/ratatui-kit/src/lib.rs`

### 宏 hygiene：所有宏展开须用绝对 `::ratatui_kit::` 路径，外部 crate 才能用

过程宏面向**第三方组件 crate**（只依赖 `ratatui-kit`、作用域里没有 `ratatui`/`crossterm` 裸名）也要能用，故 `quote!` 生成的每一处路径都必须是绝对的：`::ratatui_kit::...`，需要 ratatui 类型时写 `::ratatui_kit::ratatui::...`（经 `lib.rs` 的 `pub use ratatui`）。**不得生成裸 `ratatui::` / `crossterm::` 路径**——库内因作用域有 `ratatui` 照常编译，外部 crate 会 `cannot find crate/module ratatui` 直接炸，而库内四件套永远发现不了。

**踩过的坑**：`with_layout_style` 曾对注入字段用裸 `ratatui::layout::Margin` 等（6 处），而同宏 `layout_style()` 方法体却是对的 `::ratatui_kit::layout_style::`；调研第三方生态就绪度时用「只依赖 ratatui-kit 的外部 probe crate」一编译即暴露，已统一改为 `::ratatui_kit::ratatui::layout::*`。

**正确做法**：改任何 `quote! { ... }` 里的类型/函数路径，一律写绝对 `::ratatui_kit::`。

**回归护栏必须用「只依赖 ratatui-kit 的独立 crate」，trybuild 抓不到**：`tests/ui/pass` 用例**不能**当 hygiene 护栏——trybuild 生成的临时 crate 会 mirror 被测 crate 的 dependencies/dev-dependencies（实测 `target/tests/trybuild/ratatui-kit-tests` 的 `Cargo.toml` 直接依赖 `ratatui`/`crossterm`），故其作用域里有裸 `ratatui`，宏即便回退成裸 `ratatui::layout::…` 也能解析、编译通过（**假绿**）。当初 `with_layout_style` 的 hygiene bug 就是因此长期没被 CI 发现。真正的外部视角护栏要用一个**只**声明 `ratatui-kit` 依赖（无 `ratatui`/`crossterm`）的 crate 跑 `cargo check`——本仓库以 workspace 成员 `crates/external-api-probe`（`publish = false`）承担这个角色，随 `--workspace` 自动编译，宏 hygiene 一回退就红。

**同类:宏生成代码的 lint 也要自洽**：`with_layout_style` 生成的 `layout_style()` 里有 `..Default::default()`（只选部分布局字段时必要，全选时多余），会触发 `clippy::needless_update`。库内靠 `lib.rs` 顶部的 `#![allow(clippy::needless_update)]` 压住，但外部 crate 没有这个 crate 级 allow，`-D warnings` 就报错。已改为让宏在生成的 `layout_style()` 上自带 `#[allow(clippy::needless_update)]`。**原则:宏 `quote!` 出的代码不得依赖使用方 crate 级的 `#![allow(...)]` 或 lint 配置——需要的 allow 由宏自己带上。** 这类「外部才暴露」的问题正是 `external-api-probe` 护栏的价值。

**相关文件**：`crates/ratatui-kit-macros/src/with_layout_style.rs`、`crates/ratatui-kit/src/lib.rs`（`pub use ratatui` + `extern crate self`）、`crates/external-api-probe/`（hygiene 护栏）

## 手写 Component 与 context-aware hooks

### Modal 等手写组件经 updater 拿 SystemContext 登记输入层

`#[component]` 函数组件由宏在 implementation 前 `hooks.with_context_stack(updater.component_context_stack())` 升级为 context-aware;**手写 `Component`** 的 `update` 拿到的 `hooks.context == None`(`AnyComponent::update`,`component/mod.rs` 原样转发),直接 `hooks.use_context_mut` 会 **panic**(`use_context.rs` 的 `expect("context not available")`)。两条路：

- 需要 context-aware **hook**(如 `ScrollView` 用 `use_event_handler`/`use_input_layer`)→ 在 `update` 体内先 `let mut hooks = hooks.with_context_stack(updater.component_context_stack());`,且把 hooks 操作置于后续 `&mut updater` 操作（`set_layout_style`/`update_children`)**之前**(时序分离,否则借用冲突)。
- 只需读写 **context 数据**(如 `Modal` 登记输入层)→ 直接 `updater.get_context_mut::<SystemContext>()`(降级 `None` 版,不经 hooks)。

**借用纪律**：任何经 `SystemContext` 守卫登记 input 都要**块内即弃守卫**再 `update_children`——`SystemContext` 是全树共享单个 `RefCell`,不 drop 则子组件 `use_exit`(panic 版 `use_context_mut`)撞 `AlreadyBorrowed`。

**不在框架层统一注入**：`Component::update(props, hooks, updater)` 同时持 `hooks` 与 `&mut updater`,无法给 hooks 注入与 updater 同源的 `&ContextStack`(借用冲突);函数组件能 context-aware 是因宏把 implementation 与 update_children 时序分开。

`Modal` 新增 `layer: Option<InputLayer>` / `blocks_lower: Option<bool>` prop：外传 `layer` 时复用父级层不重复 push（「handler 在 Modal 父级」场景,需父级 `use_input_layer` + `use_event_handler(Layer(h))` + `Modal(layer: Some(h))` 三件套配对,漏传任一→父级 handler 失聪),否则自开层;均在拿到 layer id 后 drop 守卫再注入 `CurrentLayer` 给子树。

**相关文件**：`crates/ratatui-kit/src/components/modal.rs`、`crates/ratatui-kit/src/components/scroll_view/mod.rs`、`crates/ratatui-kit/src/hooks/use_input.rs`、`crates/ratatui-kit/src/component/mod.rs`
