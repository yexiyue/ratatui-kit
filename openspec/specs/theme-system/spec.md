# theme-system Specification

## Purpose
TBD - created by archiving change add-theme-system. Update Purpose after archive.
## Requirements
### Requirement: Palette 语义色板作为唯一真源

框架 MUST 提供 `Palette` 类型作为主题的唯一颜色真源,字段自底向上覆盖全部内置组件 slot 的实际需要,至少包含 `bg` / `surface` / `overlay`、`fg` / `fg_dim`、`accent` / `on_accent`、`selection`、`border` / `border_active`、`success` / `warning` / `error` / `info`、`placeholder`。`Palette` MUST 标注 `#[non_exhaustive]`;下游 MUST 经 `Default` + 字段修改或 builder 构造,MUST NOT 依赖结构体字面量穷举字段。

#### Scenario: 新增语义色不破坏下游构造

- **WHEN** 未来给 `Palette` 增加一个新语义色字段
- **THEN** 依赖 `Palette::default()` 并按需覆盖字段的下游代码仍编译通过,无需改动

#### Scenario: 覆盖全部组件不留空

- **WHEN** 用一套 `Palette` 派生全部内置组件主题
- **THEN** 每个组件的每个样式 slot 都能从 `Palette` 取到确定颜色,不出现未定义/空样式的 slot

### Requirement: ComponentTheme 派生自 Palette

每个内置组件 MUST 有对应的 `FooTheme`(如 `SelectTheme`、`TableTheme`),实现 `ComponentTheme` trait(`Clone + Default + 'static`)并暴露 `from_palette(&Palette) -> Self`。`from_palette` 中一切**颜色** MUST 取自入参 `Palette`,MUST NOT 硬编码颜色字面量;**非颜色样式决定**(高亮符号、`DIM` 遮罩、选中 `BOLD`、前景配强调底的配对)MAY 由该组件的 `from_palette` 自行承接。`FooTheme::default()` SHALL 等价于 `FooTheme::from_palette(&Palette::default())`。

#### Scenario: 组件颜色统一自 Palette

- **WHEN** 检视任一 `FooTheme::from_palette` 实现
- **THEN** 其中不含颜色字面量(如 `Color::Cyan`),所有颜色引用自 `palette` 参数

#### Scenario: default 与 from_palette 默认一致

- **WHEN** 比较 `FooTheme::default()` 与 `FooTheme::from_palette(&Palette::default())`
- **THEN** 两者产出的样式相等

### Requirement: 三级主题解析链

组件解析自身主题时 MUST 依次尝试:① 祖先注入的显式 `FooTheme` override context;② 否则 `FooTheme::from_palette(&palette)`(`palette` 取自祖先 `Palette` context);③ 二者皆无则 `FooTheme::default()`。更近(更靠栈顶)的注入 MUST 遮蔽更远的注入。

#### Scenario: 无 Provider 兜底

- **WHEN** 组件在没有任何 `PaletteProvider` / `ThemeOverride` 祖先的树中渲染
- **THEN** 组件使用 `FooTheme::default()`,不 panic

#### Scenario: override context 优先于 palette 派生

- **WHEN** 子树祖先同时存在 `Palette` context 与一个显式 `SelectTheme` override
- **THEN** 该子树内 `Select` 使用显式 `SelectTheme`,忽略 palette 派生结果

### Requirement: PaletteProvider 与 ThemeOverride 注入

框架 MUST 提供 `PaletteProvider`(注入 `Palette`)与 `ThemeOverride<T>`(注入一个 `FooTheme` override;多个 override 通过嵌套多个 `ThemeOverride` 表达)组件,底层复用 context 注入机制。二者 MUST 为透明布局节点,不占独立布局盒。

#### Scenario: 全局主题注入

- **WHEN** 用 `element!(PaletteProvider(palette: p) { App() })` 包裹应用
- **THEN** `App` 子树内所有组件按 `p` 派生主题

#### Scenario: 子树组件级覆盖

- **WHEN** 在子树包一层 `ThemeOverride` 注入自定义 `TableTheme`
- **THEN** 仅该子树的 `Table` 用自定义主题,子树外 `Table` 不受影响

### Requirement: 主题读取 hooks 与手写组件读取纪律

框架 MUST 提供 `use_palette()` 与 `use_component_theme::<T>()` 两个 Sealed hook,返回 **owned 值**(非借用守卫)。手写 `Component` 在 `update` 中经 `ComponentUpdater::get_context` 或 `Hooks::with_context_stack` 读取主题时,MUST 先 clone 并 drop 借用守卫,再调用 `update_children`,以规避 `AlreadyBorrowed` panic 与 `Ref` 对 `&mut updater` 的借用冲突。

#### Scenario: hook 返回 owned 值

- **WHEN** 组件调用 `hooks.use_component_theme::<SelectTheme>()`
- **THEN** 返回一个可自由持有的 `SelectTheme`,不残留 context 借用守卫

#### Scenario: 手写组件读取后不阻塞子树

- **WHEN** 手写组件读取主题后继续 `update_children`,且子组件也读取同类型 context
- **THEN** 不发生 `AlreadyBorrowed` panic

### Requirement: per-call 覆盖用 Option<Style> 与 patch 合成

所有组件的 per-call 样式 props MUST 为 `Option<Style>`,并接受 `impl Into<Option<Style>>`。语义:`None` = 用主题解析结果;`Some(s)` = 以 `resolved.patch(s)` 覆盖(`s` 中 `None` 字段保留主题、`Some` 字段覆盖);`Some(Style::reset())` = 清空到终端默认。组件 MUST 以 `resolved.patch(props)` 应用样式(主题在底、props 在上),MUST NOT 反向合成或整体 overwrite。

#### Scenario: 省略即用主题

- **WHEN** `element!(Select())` 未传 `highlight_style`
- **THEN** Select 高亮使用解析出的主题样式

#### Scenario: 部分覆盖保留主题其余字段

- **WHEN** 传入 `highlight_style: Some(Style::new().bold())`
- **THEN** 渲染结果为主题高亮样式叠加 BOLD,主题颜色保留

### Requirement: 内置组件默认观感统一来源

全部内置组件的默认样式 MUST 来自主题解析链,MUST NOT 在 `Props::default()` 或 draw/update 中硬编码颜色/修饰符。原样式中性、选中态默认不可见的组件(如 `TreeSelect`、`VirtualList`)的选中态 MUST 由主题提供,默认可见。

#### Scenario: Props::default 不含样式字面量

- **WHEN** 检视迁移后各组件 `Props::default()`
- **THEN** 不含 `Color::*` / `Modifier::*` 等硬编码样式字面量

#### Scenario: 中性组件选中态可见

- **WHEN** `TreeSelect` / `VirtualList` 在默认主题下渲染且存在选中项
- **THEN** 选中项呈现可见的主题高亮

### Requirement: 运行时响应式换肤

将主题源置于响应式状态(`Atom<Palette>` 或 `use_state<Palette>`)并驱动 `PaletteProvider` 时,写入新 `Palette` MUST 经 Waker 唤醒渲染循环并使整棵子树以新主题重渲。

#### Scenario: 切换 Palette 触发重渲

- **WHEN** 用户操作把驱动 `PaletteProvider` 的 `Atom<Palette>` 写入另一套配色
- **THEN** 下一帧订阅子树以新配色重绘,无需手动触发

### Requirement: 主题协议 always-on 与特性门控边界

主题协议本体(`Palette`、`ComponentTheme`、各 `*Theme`、`PaletteProvider` / `ThemeOverride`、`use_palette` / `use_component_theme`)MUST 在核心 crate always-on(被未门控组件消费,无法门控)且不引入新运行时依赖。预设主题包、`serde` 支持、`ratatui-themes` / `ratatui-themekit` 适配器 MUST 置于核心 crate 的 feature 之后,MUST NOT 另立 contrib crate。

#### Scenario: 默认特性下主题可用

- **WHEN** 一个未开启任何 feature 的下游依赖 ratatui-kit
- **THEN** `Palette` / `use_palette` / `PaletteProvider` 等协议项可直接使用

#### Scenario: 适配器随 feature 出现

- **WHEN** 开启 `ratatui-themes` 适配 feature
- **THEN** 暴露 palette 转换项(转换函数或 `From` 实现);未开启时核心不引入该依赖

### Requirement: 主题行为的离屏渲染断言

render 测试 harness MUST 能断言渲染后 buffer 的 per-cell `Style`(fg / bg / modifier)。主题相关测试 SHALL 至少覆盖:无 Provider 默认观感、`Palette` 生效、组件级 override、`Option<Style>` per-call 覆盖、`Style::reset()` 清空。

#### Scenario: 断言主题色落到 cell

- **WHEN** 在给定 `Palette` 下离屏渲染一个高亮组件
- **THEN** 对应 cell 的 `Style` 反映该 `Palette` 派生的高亮 fg / bg

