# Extension API Surface

This document defines the **public API surface that `ratatui-kit` commits to** for
third-party component / hook authors. If you are building a `ratatui-kit-<name>`
crate, depend only on the items listed under *Stable surface*.

> 本文件面向第三方组件作者,列出框架承诺遵守 semver 的公共 API。标注 `#[doc(hidden)]`
> 的项(宏协议 + 内部实现)即使是 `pub` 也**不属于**稳定面,不要直接依赖。

## Stability policy

- **`0.x` semver**: a breaking change to any *Stable surface* item bumps the **minor**
  version; purely additive changes go in a **patch**.
- **Depend with a range**, e.g. `ratatui-kit = ">=0.8, <0.9"`, matching the surface
  version you build against.
- **Get `ratatui` / `crossterm` types through the re-exports** — `ratatui_kit::ratatui`
  and `ratatui_kit::crossterm` — instead of adding a direct dependency. This avoids a
  second, possibly incompatible, copy of `ratatui` in your tree.
- Items marked `#[doc(hidden)]` are `pub` only because the macros or the runtime need
  them to be; they are **not** covered by the policy above and may change at any time.

## Stable surface (semver-guaranteed)

### Component contract
`Component`, `ComponentUpdater`, `ComponentDrawer`, `Components` (+ `get_constraints`),
`LayoutStyle`, `NoProps`, `Props`.

### Built-in components

Every `pub` component re-exported from the crate root (and its `*Props`) is part of the
stable surface — a component crate may compose them. Core: `View`, `Text`, `Border`,
`Center`, `Fragment`, `ScrollView`, `ScrollBars`, `ScrollbarVisibility`, and the `Modal`
family. Feature-gated: `Input` / `SearchInput` (`input`), `TreeSelect` (`tree`),
`VirtualList` (`virtual-list`).

Table (feature `table`): `Table`, `TableColumn`, `TableCell`, `TableCellAlignment`,
`TableBorderMode`, `TableWrapMode`, `TableState`, `RenderTableRow`.

### Elements
`Element`, `AnyElement`, `ElementExt` (`fullscreen` / `render_loop`).

### Hooks
`Hooks`, `Hook`, `Hooks::use_hook`, and the built-in hook traits:
`UseState`, `UseContext`, `UseFuture`, `UseMemo`, `UseEffect` / `UseAsyncEffect`,
`UseAsyncState`, `UseInsertBefore`, `UseTerminalSize` / `UsePreviousSize`,
`UseExit`, `UseOnDrop`, `UseInputLayer`, `UseEventHandler`,
and feature-gated `UseRouter` (`router`), `UseAtom` (`atom`).

### State
`State` (and the underlying `ReactiveHandle` + its `ReactiveRef` / `ReactiveMutRef` /
`ReactiveMutNoUpdate` guards and operator overloads), `AsyncState`.

### Theming (always-on protocol)
The theme protocol ships in every build (zero extra deps):
`Palette` (the single color source, `#[non_exhaustive]` — construct via `Palette::default()`
then set fields), `ComponentTheme` (`Clone + Default + 'static`; implement it on your own
`FooTheme` to derive component styles from a `Palette`), `UseTheme`
(`Hooks::use_palette` / `Hooks::use_component_theme`),
the inherent `ComponentUpdater::use_palette` / `ComponentUpdater::use_component_theme` (for
hand-written `Component`s that read a theme in `update`), `PaletteProvider` (inject a global
`Palette`), and `ThemeOverride<T>` (inject one component-level `FooTheme` override — needs a
turbofish, e.g. `ThemeOverride::<BorderTheme>(theme: ...)`, since `element!` does not infer a
hand-written generic component's type parameter).

Each built-in component's `FooTheme` is part of the surface too (same feature gate as the
component): always-on `TextTheme`, `BorderTheme`, `ModalTheme`, `ConfirmModalTheme`,
`AlertModalTheme`, `ShortcutInfoModalTheme`, `SelectTheme`, `MultiSelectTheme`; gated
`InputTheme` / `SearchInputTheme` (`input`), `TreeSelectTheme` (`tree`), `VirtualListTheme`
(`virtual-list`), `TableTheme` (`table`). Resolve chain per component: explicit `FooTheme`
override context → `FooTheme::from_palette(&palette)` → `FooTheme::default()`. Runtime
theming = put the `Palette` in an `Atom` / `use_state` driving `PaletteProvider` (context
reads are passive and do not subscribe on their own). Per-call style props are
`Option<Style>`, applied with the same semantics as `theme.slot.patch(prop.unwrap_or_default())`
(`None` → theme, `Some(Style::reset())` → clear to terminal default).

Feature `serde` adds `Serialize` / `Deserialize` on `Palette` (pulls `ratatui/serde`).

### Context & events
`Context`, `ContextStack` (opaque token — pass it by name, do not construct),
`Handler`, `EventResult`, `EventPriority`, `EventScope`, `EventOptions`, `InputLayer`,
`SystemContext` (its `exit()` is the escape hatch behind `use_exit`).

### Routing (feature: `router`)
`Navigate` (returned by `use_navigate`).

### Terminal
`Terminal`, `TerminalImpl`, `CrossTerminal` — backend / custom render-loop entry points.

### Global state (feature: `atom`)
`Atom`, `AtomState` (+ its guards).

### Macros
`element!`, `#[component]`, `#[derive(Props)]`, `#[with_layout_style]`,
and `routes!` (feature: `router`).

### Test utilities (feature: `test-util`)
`test_util::render_frame` / `test_util::render_frames` — offscreen-render a component
tree into a `ratatui::buffer::Buffer` (no real terminal) for integration tests, e.g.
mounting a component under `PaletteProvider` and asserting the rendered cell style
tracks the injected `Palette`. This is the same helper the core crate's own
`render/harness.rs` uses internally; extension crates should prefer it over
hand-rolling an offscreen renderer. Test-only surface — enable `test-util` as a
`dev-dependencies` feature, not in your crate's own runtime feature set.

## Not part of the surface

These are `pub` but `#[doc(hidden)]` — do not depend on them.

### Macro protocol (referenced by macro expansion only)
`ElementType`, `ExtendWithElements`, `extend_with_elements`, `ElementKey`,
`ElementRepr`, `WidgetAdapter*`, `StatefulWidgetAdapter*`, `Route::new`.

### Internal implementation
`AnyComponent`, `InstantiatedComponent`, `AnyProps`, `Tree`,
`Notifier` / `ReactiveValue` / `SingleWaker` / `WakerMap`, `LayerId`,
`UseEffectImpl` / `UseAsyncEffectImpl` / `UseFutureImpl` / `UseMemoImpl` /
`UsePreviousSizeImpl`, `UpdaterTerminal`.

## Authoring checklist

- Depend only on *Stable surface* items; reach `ratatui` / `crossterm` via
  `ratatui_kit::ratatui` / `ratatui_kit::crossterm`.
- Macros expand to absolute `::ratatui_kit::…` paths. This works out of the box when the
  dependency is named `ratatui-kit`. If you rename it via `cargo`
  (`foo = { package = "ratatui-kit" }`), add `extern crate foo as ratatui_kit;` at your
  crate root so the macro paths resolve again (a standard Rust mechanism).
- `#[component]` functions are **transparent-layout wrappers**: put layout props on the
  **returned root element**, not on the wrapper.
- Feature-gate any heavy dependency (`optional = true` + a feature); keep default
  features minimal.
- Panic / expect / error messages shown to library users are **English**.
- All examples and doctests must compile — this is the regression baseline.

## Guardrail

The example `examples/hygiene_probe.rs` lives in the `ratatui-kit-examples` crate, which
**does not depend on `ratatui` / `crossterm` directly**. It exercises the macros + a manual
`Component` + a custom `Hook`, and is compiled by `cargo test --examples`, so if a macro
ever expands to a bare `ratatui::` / `crossterm::` path (or otherwise leaks a non-exported
item), it fails to compile and CI goes red. `trybuild` cannot catch this — its temporary
crate mirrors the tested crate's `ratatui` / `crossterm` dependencies.
