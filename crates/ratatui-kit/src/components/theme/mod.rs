// theme 组件模块:主题系统协议(always-on,零新依赖)。
//
// - [`Palette`]:唯一颜色真源。
// - [`ComponentTheme`]:每组件 `FooTheme` 实现它,从 `Palette` 派生本组件样式 slot。
// - [`UseTheme`]:`use_palette` / `use_component_theme` 两个读取 Hook(被动读取,不注册 waker;
//   运行时换肤靠把 `Palette` 放进 `Atom`/`use_state` 驱动 `PaletteProvider`)。
// - [`PaletteProvider`] / [`ThemeOverride`]:注入全局 palette / 组件级 override 的组件。
//
// 解析链(每组件一致):显式 `FooTheme` override context → `FooTheme::from_palette(&palette)`
// → `FooTheme::default()`。因 `use_palette` 缺省回退 `Palette::default()`,且约定
// `FooTheme::default() == from_palette(&Palette::default())`,后两级在实现上收敛为一次 `from_palette`。

mod palette;
pub use palette::*;

use std::marker::PhantomData;

use ratatui::style::Style;
use ratatui_kit_macros::Props;

use crate::{AnyElement, Component, ComponentUpdater, Context, Hooks, UseContext};

/// 组件主题 trait:每个内置组件的 `FooTheme` 实现它,从 [`Palette`] 派生本组件样式 slot。
///
/// 约定:`Self::default()` 必须等价于 `Self::from_palette(&Palette::default())`,
/// 这样「无 Provider 兜底」与「palette 派生」在解析链上收敛为同一实现。
pub trait ComponentTheme: Clone + Default + 'static {
    /// 从共享调色板派生本组件主题。颜色一律取自 `palette`;高亮符号、`DIM`/`BOLD`
    /// 等非颜色决定由各组件在此承接。
    fn from_palette(palette: &Palette) -> Self;
}

/// 合成组件主题 slot 与 per-call 样式覆盖。
///
/// `None` 表示完全使用主题；`Some(s)` 表示 `theme.patch(s)`，其中 `Style::reset()`
/// 可清空主题落下的字段。
pub(crate) fn resolve_style(theme: Style, override_style: Option<Style>) -> Style {
    theme.patch(override_style.unwrap_or_default())
}

mod private {
    pub trait Sealed {}
    impl Sealed for crate::Hooks<'_, '_> {}
}

/// 读取主题的 Hook 扩展(函数组件开箱即用;手写 `Component` 需先
/// `hooks.with_context_stack(updater.component_context_stack())`,或改用
/// [`ComponentUpdater::use_palette`] / [`ComponentUpdater::use_component_theme`])。
pub trait UseTheme: private::Sealed {
    /// 读取当前 [`Palette`](owned 值);无 [`PaletteProvider`] 时回退 [`Palette::default`]。
    fn use_palette(&self) -> Palette;
    /// 按解析链读取某组件主题:显式 override context → `from_palette` → `default`。
    fn use_component_theme<T: ComponentTheme>(&self) -> T;
}

impl UseTheme for Hooks<'_, '_> {
    fn use_palette(&self) -> Palette {
        // try_use_context 非 panic;拿到守卫立即拷贝出 owned 值,不残留借用。
        self.try_use_context::<Palette>()
            .map(|p| *p)
            .unwrap_or_default()
    }

    fn use_component_theme<T: ComponentTheme>(&self) -> T {
        match self.try_use_context::<T>() {
            Some(t) => t.clone(),
            None => T::from_palette(&self.use_palette()),
        }
    }
}

impl ComponentUpdater<'_, '_> {
    /// 手写 `Component` 在 `update` 中读取当前 [`Palette`](owned 值,读后即弃守卫)。
    pub fn use_palette(&self) -> Palette {
        self.get_context::<Palette>()
            .map(|p| *p)
            .unwrap_or_default()
    }

    /// 手写 `Component` 在 `update` 中按解析链读取某组件主题(owned 值,读后即弃守卫)。
    ///
    /// 借用纪律:返回 owned 值后守卫即 drop,可安全继续 `update_children`。
    pub fn use_component_theme<T: ComponentTheme>(&self) -> T {
        match self.get_context::<T>() {
            Some(t) => t.clone(),
            None => T::from_palette(&self.use_palette()),
        }
    }
}

/// 全局主题注入:向子树提供一个 [`Palette`],子树内组件据此派生各自主题。
///
/// 透明布局节点,不占独立布局盒。运行时换肤:把 `palette` 由 `Atom<Palette>` / `use_state`
/// 驱动,写入即唤醒整树重渲。
#[derive(Default, Props)]
pub struct PaletteProviderProps<'a> {
    /// 子元素列表。
    pub children: Vec<AnyElement<'a>>,
    /// 注入的调色板。
    pub palette: Palette,
}

/// 见 [`PaletteProviderProps`]。
pub struct PaletteProvider;

impl Component for PaletteProvider {
    type Props<'a> = PaletteProviderProps<'a>;

    fn new(_props: &Self::Props<'_>) -> Self {
        Self
    }

    fn update(
        &mut self,
        props: &mut Self::Props<'_>,
        _hooks: Hooks,
        updater: &mut ComponentUpdater,
    ) {
        updater.set_transparent_layout(true);
        let mut ctx = Context::owned(props.palette);
        updater.update_children(props.children.iter_mut(), Some(ctx.borrow()));
    }
}

/// 组件级主题覆盖:向子树注入某个 `FooTheme` override,只影响该类型组件(解析链第一级)。
///
/// 手写泛型组件不会从 `theme:` prop 推断类型,调用时需显式写出主题类型:
/// `element!(ThemeOverride::<BorderTheme>(theme: my_border_theme) { ... })`。透明布局节点。
#[derive(Props)]
pub struct ThemeOverrideProps<'a, T>
where
    T: ComponentTheme,
{
    /// 子元素列表。
    pub children: Vec<AnyElement<'a>>,
    /// 注入的组件主题 override。
    pub theme: T,
}

impl<T> Default for ThemeOverrideProps<'_, T>
where
    T: ComponentTheme,
{
    fn default() -> Self {
        Self {
            children: Vec::new(),
            theme: T::default(),
        }
    }
}

/// 见 [`ThemeOverrideProps`]。
// 用 `fn() -> T` 而非裸 `T`:不让标记字段把 `T` 的 Unpin/auto-trait/drop 约束传染给组件。
pub struct ThemeOverride<T>(PhantomData<fn() -> T>);

impl<T> Component for ThemeOverride<T>
where
    T: ComponentTheme,
{
    type Props<'a> = ThemeOverrideProps<'a, T>;

    fn new(_props: &Self::Props<'_>) -> Self {
        Self(PhantomData)
    }

    fn update(
        &mut self,
        props: &mut Self::Props<'_>,
        _hooks: Hooks,
        updater: &mut ComponentUpdater,
    ) {
        updater.set_transparent_layout(true);
        let mut ctx = Context::owned(props.theme.clone());
        updater.update_children(props.children.iter_mut(), Some(ctx.borrow()));
    }
}
