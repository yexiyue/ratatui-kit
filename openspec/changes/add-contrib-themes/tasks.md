## 1. Contrib workspace baseline

- [x] 1.1 Update `ratatui-kit-contrib` workspace dependency policy from `ratatui-kit = ">=0.9, <0.10"` to `ratatui-kit = ">=0.10"` and update README guidance to remove the `<0.11` upper-bound recommendation.
- [x] 1.2 Update `crates/ratatui-kit-markdown/Cargo.toml` to depend on `ratatui-kit >=0.10` and keep using `ratatui_kit::ratatui` / `ratatui_kit::crossterm` re-exports for public types.
- [x] 1.3 Run the existing contrib workspace checks after the baseline bump to confirm `ratatui-kit-markdown` still compiles before adding the new crate.

## 2. ratatui-kit-themes crate

- [x] 2.1 Add `crates/ratatui-kit-themes` as a workspace member with package metadata matching contrib conventions (`ratatui-kit-themes`, official description, keywords, license, README, changelog/cliff setup if needed).
- [x] 2.2 Add dependencies: `ratatui-kit = ">=0.10"` and `ratatui-themes = { version = "0.2", default-features = false }`; verify no `ratatui-themekit` dependency or feature exists.
- [x] 2.3 Implement public conversion API: re-export `Theme`, `ThemeName`, `ThemePalette`; add `palette_from_name`, `palette_from_theme_palette`, and local `IntoKitPalette` implementations for supported upstream types.
- [x] 2.4 Implement deterministic `ThemePalette -> Palette` mapping, including `on_accent` contrast inference and faithful default use of `ThemePalette.bg`.
- [x] 2.5 Implement terminal-background helper that resets `bg` / `surface` / `overlay` to `Color::Reset` while preserving the rest of the mapped palette.
- [x] 2.6 Add unit tests covering every `ThemeName::all()` value, explicit field mapping, `on_accent` inference sanity, terminal-background helper behavior, and absence of `ratatui-themekit` from dependency/features.

## 3. ratatui-kit-markdown theme integration

- [x] 3.1 Add markdown-local theme types (`MarkdownTheme`, `CodeBlockTheme`, `BlockquoteTheme`, `DividerTheme`, `DiffTheme`) that implement `ComponentTheme::from_palette(&Palette)`.
- [x] 3.2 Convert `Blockquote` defaults from hardcoded colors to theme resolution; keep `prefix_color` / `bg_color` or replacement style props as `Option<Style>`-compatible overrides.
- [x] 3.3 Convert `Divider` defaults to theme resolution while preserving per-call override semantics and existing layout forwarding.
- [x] 3.4 Convert `CodeBlock` defaults to theme resolution for line numbers, code text, border, and language label; keep syntect highlighting behavior intact and use themed fallback styles when syntax highlighting is unavailable.
- [x] 3.5 Convert `Diff` defaults to theme resolution for add/remove/unchanged/line-number styles, deriving semantic colors from `Palette.success` / `Palette.error` / `Palette.fg_dim`.
- [x] 3.6 Refactor Markdown rendering so heading markers, list markers, rules, table borders, inline code, and links derive from `MarkdownTheme` rather than parser-time hardcoded colors.
- [x] 3.7a Add unit tests proving each component's theme derivation/override logic (`ComponentTheme::from_palette`, `resolve_style`, `Some(Style)` / `Some(Style::reset())` semantics) is correct in isolation.
- [x] 3.7b Add an integration test that actually mounts a component under a real `element!(PaletteProvider { ... })` tree and asserts runtime palette switching updates defaults end-to-end through `use_component_theme`/context resolution. Unblocked by `ratatui-kit` core's new `test-util` feature (`test_util::render_frame`, released as `ratatui-kit v0.10.1`), which exposes the same offscreen-render harness `render/harness.rs` uses internally. Landed as `crates/ratatui-kit-markdown/tests/palette_provider.rs` (contrib repo), covering `Markdown`/`Divider`/`Blockquote`.

## 4. Gallery example and recording

- [x] 4.1 Add a `ratatui-kit-themes` gallery example that uses `PaletteProvider` and shows palette swatches, core components, and `ratatui-kit-markdown` previews in one screen.
- [x] 4.2 Add gallery interactions: `t`/`T` cycle `ratatui_themes::ThemeName`, `b` toggles theme-background vs terminal-background mode, and `q`/`Q` exits.
- [x] 4.3 Include representative preview states: selected row/list/table state, status colors, border focus, markdown text, inline code/link, code block, blockquote, divider, and diff.
- [x] 4.4 Add a package-local VHS tape and generated GIF asset for the gallery; prebuild examples before recording and run VHS from the contrib repo root.
- [x] 4.5 Use VHS `Screenshot` verification frames for at least one dark theme and one light theme, confirming no compile logs/shell prompts and checking contrast for selection, borders, markdown inline code/link, and diff.

## 5. Documentation and validation

- [x] 5.1 Update `ratatui-kit-contrib` README crate table and recording instructions to include `ratatui-kit-themes`.
- [x] 5.2 Add `ratatui-kit-themes` README usage examples showing `ThemeName::...into_kit_palette()` with core `PaletteProvider` and terminal-background helper.
- [x] 5.3 Update `ratatui-kit-markdown` README/docs to describe theme support and per-call override semantics.
- [x] 5.4 Run contrib validation: `cargo fmt --all --check`, `cargo clippy --all-targets --all-features --workspace -- -D warnings`, `cargo test --all-features --workspace --lib --tests --examples`, and `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items --all-features --workspace --examples`.
- [x] 5.5 Verify dependency graph: no `ratatui-themekit`, no direct public-use `ratatui` dependency in contrib crates beyond what upstream dependencies bring transitively.
- [x] 5.6 Update main repo docs/spec references if implementation changes any public theme-system wording beyond the OpenSpec delta.
