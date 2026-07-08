## 1. 主题协议基座

- [x] 1.1 新建 `components/theme` 模块:定义 `Palette`(`#[non_exhaustive]`,字段 `bg`/`surface`/`overlay`、`fg`/`fg_dim`、`accent`/`on_accent`、`selection`、`border`/`border_active`、`success`/`warning`/`error`/`info`、`placeholder`)与 `Palette::default()`(新统一配色)
- [x] 1.2 定义 `ComponentTheme` trait(`Clone + Default + 'static`):`fn from_palette(palette: &Palette) -> Self`;约定 `FooTheme::default() == from_palette(&Palette::default())`
- [x] 1.3 实现 `use_palette()` 与 `use_component_theme::<T: ComponentTheme>()` 两个 Sealed hook,返回 owned 值(内部 `use_context` 后 clone、drop 守卫);解析链:显式 `T` override context → `T::from_palette(palette)` → `T::default()`
- [x] 1.4 实现 `PaletteProvider`(注入 `Palette`)与 `ThemeOverride<T>`(注入一个 `FooTheme` override;多个 override 可嵌套),复用 context 注入机制,均为透明布局节点
- [x] 1.5 在 `prelude` 增出 `Palette`/`ComponentTheme`/`PaletteProvider`/`ThemeOverride`/`use_palette`/`use_component_theme` 及各 `*Theme`
- [x] 1.6 提供手写组件读取模板:经 `get_context`/`with_context_stack` 读取后 clone、drop 守卫、再 `update_children`(见 `ComponentUpdater::use_palette`/`use_component_theme`)

## 2. render harness 样式断言能力

- [x] 2.1 给 `render/harness.rs` 增加 per-cell `Style` 读取/断言 helper(fg/bg/modifier),不改其 `#[cfg(test)]` 私有性(`cell_style`)
- [x] 2.2 增加通用断言用例骨架:无 Provider 默认观感 / `Palette` 生效 / 组件级 override / `Option<Style>` 覆盖 / `Style::reset()` 清空(`theme_tests`,5 例全绿)

## 3. 低层组件迁移(Text / Border / Input / Modal)

- [x] 3.1 为各组件定义 `TextTheme`/`BorderTheme`/`InputTheme`/`ModalTheme` 并实现 `from_palette`(颜色取自 `Palette`,`Modal` 的 `DIM` 遮罩等非颜色决定内置于此)
- [x] 3.2 样式 props 改为 `Option<Style>`(`impl Into<Option<Style>>`);apply 路径由 overwrite 改为 `resolved.patch(props)`
- [x] 3.3 移除各 `Props::default()` 中的样式字面量;`draw`/`update` 内改经解析链取样式
- [x] 3.4 harness 断言:三组件在默认 `Palette`、显式 `Palette`、`Option<Style>` 覆盖下的 cell 样式

## 4. 组合输入组件(SearchInput / ConfirmModal / AlertModal / ShortcutInfoModal)

- [x] 4.1 定义 `SearchInputTheme`/`ConfirmModalTheme`/`AlertModalTheme`/`ShortcutInfoModalTheme` 并 `from_palette`:把状态色映射到 `success`/`error`/`accent`/`placeholder`,`DIM` 遮罩(委托 `Modal` 的 `ModalTheme`)、选中 `BOLD` 由组件承接
- [x] 4.2 样式 props 改 `Option<Style>` + `patch` 应用;移除 `Props::default()` 硬编码(yellow/green/red/dim/cyan+bold)
- [x] 4.3 保留 `ConfirmModal` 的 prop 派生逻辑(`selected_button_label_style`/`button_border_style`),改为从解析后的主题样式派生
- [x] 4.4 harness 断言:SearchInput 非激活边框/占位取自主题;confirm 默认选中按钮为主题强调 + BOLD(激活/成功/失败态需事件派发,harness 不覆盖)

## 5. 选择组件(Select / MultiSelect / TreeSelect / VirtualList)

- [x] 5.1 定义 `SelectTheme`/`MultiSelectTheme`/`TreeSelectTheme`/`VirtualListTheme` 并 `from_palette`:高亮为"`on_accent` 前景 + `selection` 底"的配对(`VirtualList` 选中由 `render_item` 自绘,仅提供基础 `style`)
- [x] 5.2 样式 props 改 `Option<Style>` + `patch`;移除硬编码
- [x] 5.3 `TreeSelect` 补默认可见选中态(主题提供,默认 `bg=selection`);保留其节点符号等非样式字面量
- [x] 5.4 harness 断言:Select 高亮取自 palette;TreeSelect 默认主题下选中可见

## 6. 表格(Table)

- [x] 6.1 定义 `TableTheme` 并 `from_palette`:header/footer/highlight 用 `accent`,border/separator 用 `border`,选中前景用 `on_accent`(注意 Table 原本无 yellow);列/单元格高亮默认留空
- [x] 6.2 顶层样式 props 改 `Option<Style>` + `patch`;复用表格已有的 cell/column/row `patch` 分层;移除 `Props::default()` 硬编码(cyan/darkgray/black);`build` 置占位、`update` 经主题解析后写入(估高与样式无关)
- [x] 6.3 harness 断言:表头(accent)/外框(border)/选中行(selection)样式取自主题

## 7. 运行时换肤与响应式

- [x] 7.1 验证 `Atom<Palette>` 驱动 `PaletteProvider`:探针 `use_atom(&PROBE_PALETTE)` + 手动两帧,帧间写入(`runtime_theme_tests`)
- [x] 7.2 harness 测试:写 `Atom<Palette>` 后下一帧 cell 边框色随之由 DarkGray → Red

## 8. 特性接线、示例与文档

- [x] 8.1 `crates/ratatui-kit/Cargo.toml`:协议本体 always-on(零新依赖);新增 `serde` feature(`Palette` 序列化,透传 `ratatui/serde`)。**未加空壳 `themes`/适配器 feature 位**——按用户「简洁」原则,避免无内容的死 feature,待真有预置/适配器再加(YAGNI)
- [x] 8.2 门控组件的主题类型随其 feature 门控:`InputTheme`/`SearchInputTheme`(input)、`TreeSelectTheme`(tree)、`VirtualListTheme`(virtual-list)、`TableTheme`(table)在各自门控模块内,不进 always-on 路径
- [x] 8.3 新增 `examples/components/theme.rs`:全局主题、运行时切换(`Atom<Palette>`+`t` 键)、`ThemeOverride::<BorderTheme>` 组件级覆盖、`Option<Style>` per-call、`Style::reset()`;注册 `[[example]] name="theme"`(required-features atom)
- [x] 8.4 更新 `EXTENSION_API.md`:新增 Theming 稳定面章节(Palette/ComponentTheme/UseTheme/PaletteProvider/ThemeOverride/各 *Theme + serde feature)
- [x] 8.5 更新 `dev-notes/knowledge/`(hooks-and-state + macros-and-props)记录主题协议、解析链、手写组件读取纪律与 turbofish 坑

## 9. 回归底线

- [x] 9.1 `cargo test --locked --all-features --workspace --lib --tests --examples` 通过(126 lib + 1 doctest;含 20 例主题断言)
- [x] 9.2 `cargo clippy --all-targets --all-features --workspace -- -D warnings` 通过
- [x] 9.3 `cargo fmt --all --check` 通过
- [x] 9.4 `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items --all-features --workspace --examples` 通过
