# Authoring a ratatui-kit Component Crate

A practical guide to building and publishing a third-party `ratatui-kit-<name>`
component crate. See [`EXTENSION_API.md`](./EXTENSION_API.md) for the exact API surface
you are allowed to depend on.

> 本指南面向第三方组件作者:如何用 ratatui-kit 的公共扩展 API 写一个独立组件 crate 并发布。

## Quick start

```bash
# 推荐:从官方模板生成
cargo generate ratatui-kit-org/ratatui-kit-component-template

# 或手动:在 Cargo.toml 里
# ratatui-kit = ">=0.7, <0.8"
```

## The component contract

### Define a component

Two ways — both implement the `Component` contract:

```rust
use ratatui_kit::prelude::*;

// A) function component (a transparent-layout wrapper)
#[component]
pub fn Badge(props: &BadgeProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    // NOTE: put layout props on the RETURNED root element, not on the wrapper.
    element!(View(gap: 1) { /* ... */ })
}

// B) manual impl for custom layout / draw
pub struct MyList;
impl Component for MyList {
    type Props<'a> = MyListProps<'a>;
    fn new(_props: &Self::Props<'_>) -> Self { Self }
    fn update(&mut self, props: &mut Self::Props<'_>, hooks: Hooks, updater: &mut ComponentUpdater) { /* ... */ }
    // override draw / calc_children_areas for custom layout
}
```

### Props

```rust
#[with_layout_style]            // adds width/height/flex_direction/... + layout_style()
#[derive(Default, Props)]       // Props is required; Default lets element! omit fields
pub struct BadgeProps<'a> {
    pub label: String,
    pub children: Vec<AnyElement<'a>>,
}
```

Components without props use the built-in `NoProps`.

### Custom hooks

Define a struct implementing `Hook`, expose it via an extension trait that calls
`hooks.use_hook(...)` (you cannot use the internal `Sealed` bound, and you don't need to):

```rust
pub struct TickHook { n: u32 }
impl Hook for TickHook {}

pub trait UseTick { fn use_tick(&mut self) -> u32; }
impl UseTick for Hooks<'_, '_> {
    fn use_tick(&mut self) -> u32 {
        let h = self.use_hook(|| TickHook { n: 0 });
        h.n += 1; h.n
    }
}
```

Hooks are indexed by call order — never put one behind a condition or loop.

## Rules & conventions

- **Depend only on the Extension API** (`EXTENSION_API.md`). Reach `ratatui` / `crossterm`
  types through `ratatui_kit::ratatui` / `ratatui_kit::crossterm` — do **not** add a direct
  `ratatui` dependency (avoids a second, incompatible copy).
- **Transparent-layout trap**: `#[component]` functions don't occupy their own layout node,
  so layout props must live on the returned root element.
- **Feature-gate heavy dependencies** (`optional = true` + a feature); keep default features
  minimal so `cargo add your-crate` stays light.
- **English** panic / expect / error messages shown to library users (comments can be any
  language).
- **Compile baseline**: all examples and doctests must compile — treat it as your CI gate.

## Renamed dependency escape hatch

Macros expand to absolute `::ratatui_kit::…` paths, which resolve as long as the dependency
is named `ratatui-kit`. If you rename it via `cargo`:

```toml
rk = { package = "ratatui-kit", version = ">=0.7, <0.8" }
```

add this at your crate root so the macro paths resolve again:

```rust
extern crate rk as ratatui_kit;
```

## Publishing

- **Name**: `ratatui-kit-<name>` (e.g. `ratatui-kit-markdown`).
- **`Cargo.toml`**: `keywords = ["ratatui-kit", "tui"]`, a relevant `categories`, and a
  version range `ratatui-kit = ">=0.7, <0.8"` matching the API surface you build against.
- **Release**: bump version → tag → CI `cargo publish` (mirror the main repo's flow).
- List your crate in [`awesome-ratatui-kit`](https://github.com/ratatui-kit-org/awesome-ratatui-kit)
  so people can find it.

## Pre-publish checklist

- [ ] Depends only on the Extension API surface
- [ ] Heavy deps are `optional` + feature-gated; default features minimal
- [ ] Runtime panic / error messages are English
- [ ] All examples and doctests compile
- [ ] `ratatui-kit` pinned with a version range
- [ ] `keywords` include `ratatui-kit`
- [ ] Layout props on the returned root element (transparent-layout)
