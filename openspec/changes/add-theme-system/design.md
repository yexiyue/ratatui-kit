## Context

现状(经代码核查确认):

- **颜色各自为政**:硬编码样式落在恰好 7 个 `Props::default()` 里 —— `select.rs:59-60`、`multi_select.rs:66-68`、`table/component.rs:80-93`、`search_input.rs:69-76`、`alert_modal.rs:46-48`、`confirm_modal.rs:51/56-58`、`shortcut_info_modal.rs:85/90`。惯例是黑字配强调底(`.fg(Black).bg(Cyan)`)、状态色 yellow/green/red、边框 DarkGray、modal 遮罩 `.dim()`、confirm 选中 `Cyan+BOLD`。注意 `Table` 用 Cyan/DarkGray/Black(**无 yellow**)。
- **无主题概念**:框架没有 `Theme`/`Palette` 任何符号(grep 为空)。每个样式都已是 `pub ...: Style` prop,今天以 **overwrite** 直接喂给 ratatui widget(如 `List::highlight_style(props.x)`)。
- **可复用的基础设施**:`ContextProvider`(`components/context_provider.rs`,`Context::owned`)、`use_context`(返 `Ref<'a,T>`)、`ComponentUpdater::get_context`(返 `Option<Ref>`,非 panic)、`Hooks::with_context_stack`(public);context 按 `TypeId` 键控、就近遮蔽。`ratatui 0.30.1` 的 `Style` 为增量合成模型:`default()` 全 `None`(不改)、`reset()` 清空到终端默认、`patch(other)=other.or(self)`。`Table` 内部已在用 `patch` 分层(`table/render.rs:182/199/243`)。
- **约束**:单线程运行时(handler/主题免 `Send+Sync`);响应式仅由 `use_state`/`Atom` 写入经 Waker 唤醒渲染循环触发;context 本身**惰性**、不挂 waker。本变更明确**不考虑向后兼容**。

## Goals / Non-Goals

**Goals:**

- 全部组件默认观感从**单一 `Palette`** 派生,颜色成体系、可一处统一。
- 主题**解耦**:无中央 god-struct;每组件拥有自己的主题类型;门控组件的主题随其 feature 进出。
- 第三方组件**一等公民**:同一套 `use_palette` / `from_palette` 机制接入,且自动与内置视觉协调。
- 运行时**响应式换肤**:`Atom<Palette>` 写入即整树重渲。
- API 无静默陷阱:三种覆盖意图(用主题 / 覆盖 / 清空)全部类型可表达。

**Non-Goals:**

- 不做兼容层、不做 `Theme::legacy()`、不保留旧硬编码观感。
- v1 不含成套预设主题与 `ratatui-themes`/`ratatui-themekit` 适配器(列为后续 feature)。
- 不新建 `ratatui-kit-contrib` crate。
- 不引入亮/暗双模(单档配色);多模留作后续 `Palette` 演进。

## Decisions

### D1:主题数据模型用 per-component context,而非中央 `Theme` struct

每个组件有自己的 `FooTheme`(`SelectTheme`/`TableTheme`/...),经 context(按 `TypeId` 键控)读取,而非把所有组件塞进一个 `Theme { select, table, ... }`。

- **理由**:① 解耦——加/改一个组件不牵动中央结构、其 `Default` 与所有全量构造点;② 第三方组件天然有位置(定义 `MyWidgetTheme`、读自己的 context,零核心改动);③ 与 **feature flag 天生咬合**——`input`/`tree`/`router` 门控组件自带门控主题类型,避免中央 `Theme` 里写 `#[cfg(feature=...)]` 字段。
- **备选**:(a) 单一 `Arc<Theme>` struct(MUI 式,心智最简、可整体序列化,但门控字段丑陋、第三方二等、加 slot 破坏 SemVer);(b) 混合 struct + `TypeId` 逃生舱(两套读法、API 面更大)。均因牺牲解耦/门控咬合被否。

### D2:共享 `Palette` 为唯一颜色真源,slot 由 `from_palette` 派生

`Palette` 持全部语义色;每个 `FooTheme::from_palette(&Palette)` 把语义色映射进本组件 slot。

- **理由**:D1 的天然写法"每组件一个 `FooTheme::default()`"若各自手挑颜色,等于把今天 7 处乱局搬进 7 个 `default()`,**统一目标落空**。以共享 `Palette` 为唯一色源既保解耦读取、又保来源统一;第三方从同一 `Palette` 取色,自动与内置协调。
- **Palette 自底向上、恰好够用**:从现有 slot 的实际需要反推,而非照搬 `ratatui-themes` 的 9 色(经批判确认那套无 `surface`/`overlay`/`selection`/`on_accent`/`placeholder`,装不下 Modal 遮罩、Table 选中行、SearchInput 占位)。
- **颜色 vs 非颜色**:`Palette` 只装颜色;高亮符号、`DIM` 遮罩、选中 `BOLD`、黑字配强调底的**配对与修饰**由各组件 `from_palette` 承接。故 `Palette` 统一颜色,组件保留结构自主。

### D3:三级解析链 override context → from_palette → default

组件解析顺序:显式 `FooTheme` override context → `FooTheme::from_palette(palette)` → `FooTheme::default()`(= `from_palette(&Palette::default())`)。

- **理由**:一条链同时满足全局主题(注 `Palette`)、组件级子树覆盖(注 `FooTheme`)、无 Provider 兜底(default)。`default()` 兜底解决 `use_context` 找不到即 panic 的问题——不靠"根部强制注入",各组件自带兜底。

### D4:per-call 覆盖用 `Option<Style>` + `resolved.patch(props)`

props 样式字段为 `Option<Style>`(`impl Into<Option<Style>>` 保留裸写)。`None`=用主题;`Some(s)`=`resolved.patch(s)`;`Some(reset())`=清空。

- **理由**:`Style::default()` 在每个 ratatui 用户眼里都是"没设",若重载成"继承主题",程序化算出的空 `Style` 会**静默继承整套主题**、编译器不报——这是 call-site 不可见的 footgun。`Option<Style>` 让三种意图全类型可表达。
- **方向承重**:合成必须 `theme.patch(props)`(主题在底、props 在上);反向会全错。`patch` 优于手写 resolver,因为它正确合成 `sub_modifier`(手写 `theme.add | props.add` 无法移除主题修饰符)。
- **备选**:(a) `Style::default()` 哨兵(文档原案,footgun,否);(b) 砍掉 per-call 样式 props、只靠 scoped provider(props 最净但一次性微调太重,否)。

### D5:不要兼容,统一替换

移除全部硬编码默认、不留 `legacy()`。

- **理由**:用户明确"颜色不统一、全部替换、保证架构最简洁"。保留兼容会逼出 `legacy()`(一整套旧审美进核心,违背"核心不塞审美")+ `Style::default()` 哨兵 + 破坏性分析等一整条支线。放弃兼容把设计空间打开,`Theme::default()` 直接 = 新设计的统一主题。

### D6:核心 + feature flag,不新建 contrib crate;协议 always-on

协议本体在核心 always-on;`serde`/预设/适配器在核心的 feature 之后。

- **理由**:① 协议被 `Text`/`Border`/`Modal` 等**未门控**组件消费,无法门控;且零新依赖,也无需门控。② 适配器放核心 feature 后,`ratatui_kit::Palette` 是**本地类型**,`From`/`.into()` 合法——文档担心的 orphan rule 直接消失。③ 单人维护 + 打 tag 发布下,第二个 crate 是持续的版本/CHANGELOG/发布顺序/docs 成本。
- **备选**:新建 `ratatui-kit-contrib`(隔离彻底但成本高、orphan rule 仍在,否)。

### D7:运行时换肤靠 `Atom<Palette>`,不靠 context 自身

context 惰性、不挂 waker;把 `Palette` 放 `Atom`/`use_state` 并驱动 `PaletteProvider`,写入经 Waker 唤醒整树重渲、Provider 重注入、子树重派生。

- **理由**:这是本运行时唯一的响应式路径。主题切换的"难点"正在此,必须显式规定 + harness 测,不能留作隐含假设。

### D8:手写组件读取纪律

`Border`/`Table`/`Modal` 等手写 `Component` 在 `update` 中经 `get_context`/`with_context_stack` 读取,**先 clone、drop 守卫,再 `update_children`**。

- **理由**:`use_context` 返 `Ref` 守卫,攥着往下递会触发 `AlreadyBorrowed` panic;且 `get_context` 的 `Ref` 借 `&self`,`update_children` 要 `&mut self`,不 drop 就借用冲突。`use_palette`/`use_component_theme` hook 内部 clone 后返 owned 值封装此纪律。

### D9:`#[non_exhaustive]` + Default/builder 构造

`Palette` 与每个 `*Theme` 标 `#[non_exhaustive]`,经 `Default` + 字段改或 builder 构造。

- **理由**:主题 slot 明确会逐版本增长;无 `#[non_exhaustive]` 则每加一色/一 slot 都是对下游字面量构造的 SemVer 破坏。代价:下游不能用结构体字面量穷举字段——可接受。

## Risks / Trade-offs

- **[大面 BREAKING]** 7 组件默认色移除 + 全部样式 props 改 `Option<Style>` + apply 路径 overwrite→patch 重写 → 缓解:一次性纳入本 change,`element!` 侧靠 `Into<Option<Style>>` 保留裸写手感;逐组件 harness 断言护航。
- **[中性组件行为变]** `TreeSelect`/`VirtualList` 选中态从"默认不可见"变"默认可见" → 缓解:在 spec 与 CHANGELOG 显式声明,示例演示。
- **[每组件解析样板]** 各组件都要走解析链 → 缓解:封 `use_palette`/`use_component_theme` 一次性收口;手写组件另有读取纪律模板。
- **[可发现性下降]** "主题旋钮"散在各 `*Theme`,不像单 struct 一眼看全 → 缓解:命名约定 `*Theme` + prelude 增出 + 文档集中列举。
- **[适配器兼容性]** 曾因 `tui-textarea` 钉死 0.29 下线 textarea → 已核实 `ratatui-themes` v0.2.0 与 `ratatui-themekit` v0.6.1 均兼容 ratatui 0.30,适配器后续 feature 无此坑。
- **[buffer 测不到 None/Some 区分]** 渲染后 cell 恒为具体色,harness 只能断言"最终合成色",测不到中间 `Style` 的 `None` 字段 → 缓解:测试改为断言"主题色/覆盖色是否落到 cell",而非内省中间态。

## Migration Plan

按风险从低到高、逐组件迁移,每步以 harness 断言收尾:

1. **协议落地**:`components/theme` 模块 —— `Palette`(`#[non_exhaustive]`)、`ComponentTheme` trait、`PaletteProvider`/`ThemeOverride`、`use_palette`/`use_component_theme`;prelude 增出。
2. **低层组件**:`Text`/`Border`/`Input`/`Modal`(本已中性,最便宜)接入解析链 + props 改 `Option<Style>`。
3. **组合输入**:`SearchInput`/`ConfirmModal`/`AlertModal`/`ShortcutInfoModal`(把硬编码状态色/DIM/BOLD 搬进各 `from_palette`)。
4. **选择组件**:`Select`/`MultiSelect`/`TreeSelect`/`VirtualList`(黑字配强调底的配对;中性组件补默认选中态)。
5. **表格**:`Table` 的 header/footer/row/highlight/border/separator slot(复用其已有 `patch` 分层)。
6. **示例与文档**:`examples/theme.rs`(全局主题 / 组件级 override / 运行时切换 / `Option<Style>` / `reset`);扩展 API 文档增列主题稳定面。
7. **特性接线**:Cargo.toml 加 `serde`/`themes`/适配器 feature(协议本体 always-on)。

回滚策略:本 change 为一体化 BREAKING,回滚即整体 revert;无分阶段发布中间态。

## Open Questions

- `Palette` 首版的确切字段与默认取值(新统一配色的具体色值)在实现期定,spec 只约束"覆盖所有 slot 不留空"。
- `ThemeOverride<T>` 一次只注入一个 `FooTheme`;需要多个组件级覆盖时嵌套多个 `ThemeOverride`。
- `serde` 序列化以 `Palette` 为单位(足以复原派生结果);override 层是否需序列化留待有需求再定。
- 后续:预设主题包与 `ratatui-themes`/`ratatui-themekit` 适配器的 feature 命名与 `From`/转换函数形态(不在 v1)。
