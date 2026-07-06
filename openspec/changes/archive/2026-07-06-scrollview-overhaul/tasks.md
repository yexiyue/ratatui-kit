## 1. Geometry foundation & nesting safety (D1, D4)

- [x] 1.1 Change `UseScrollImpl` to hold `block: Option<Block<'static>>` (drop `has_block: bool`) and compute `self.area = block.inner(drawer.area)` in `pre_component_draw`, so it matches `draw()`'s inner exactly (partial borders / padding / titles included).
- [x] 1.2 In `calc_children_areas`, save the enclosing `drawer.scroll_buffer` before setting this level's buffer; in `post_component_draw`, restore it after blitting. Replace `take().unwrap()` with a guarded `if let Some(buf) = drawer.scroll_buffer.take()`.
- [x] 1.3 Add a render-harness test: a bordered `ScrollView` reserves both side borders (no content on border cells); and a `ScrollView` nested inside another renders both windows without panic.

## 2. over_border switch + viewport-aware scrolling (D2, D3)

- [x] 2.1 Add `over_border: bool` to `Scrollbars` (default `true`); thread it through `layout_for` / `render_ref` / scrollbar rendering.
- [x] 2.2 Implement the two geometries: `over_border=true` → viewport = `inner`, scrollbar tracks on the border ring (right column / bottom row), both axes symmetric, blit clipped to `inner` so borders survive where no scrollbar; `over_border=false` → scrollbars inside `inner`, viewport = `inner` minus shown scrollbar thickness. No-block (or missing side border) → inset fallback.
- [x] 2.3 In `render_ref`, resolve `show_horizontal/show_vertical` first, then clamp offset against the viewport (`content - viewport`), fixing "last row/column unreachable when a scrollbar is shown".
- [x] 2.4 Set `state.page_size` from the post-scrollbar viewport (reuse `layout.visible_area`), not the raw area, so page scroll keeps the 1-row/column overlap.
- [x] 2.5 Render-harness tests: over_border on/off × content overflowing one/both axes; last row reachable with a horizontal scrollbar shown; page-down overlap correct.

## 3. State capability sync (is_at_bottom, getters, scroll-to-visible)

- [x] 3.1 Add `pub const fn size(&self) -> Option<Size>` and `pub const fn page_size(&self) -> Option<Size>` getters to `ScrollViewState`.
- [x] 3.2 Port `is_at_bottom()` from upstream (true before first render; else last content row within the viewport) now that `page_size` is viewport-correct; port its unit test.
- [x] 3.3 Add `scroll_to_visible(y, height)` / `ensure_visible(y_range)` that only adjusts the offset when the target is outside `[offset, offset+page_size)`; unit + harness test.

## 4. API consistency & event consumption (BREAKING) (D5, D6)

- [x] 4.1 Rename type `ScrollBars` → `Scrollbars` and prop `scroll_bars` → `scrollbars`; update `mod.rs`/`scrollbars.rs` and re-exports.
- [x] 4.2 Rename prop `scroll_view_state` → `state`; rename `disabled: bool` → `active: bool` (default `true`).
- [x] 4.3 Make the two modes orthogonal: `let state = props.state.unwrap_or(internal)`; gate built-in scrolling solely on `active` (remove the `state.is_none()` guard) so an external state handle keeps built-in scrolling.
- [x] 4.4 Change `ScrollViewState::handle_event` to return `EventResult` (or `bool` mapped to it); the built-in handler returns `Consumed` for input it scrolled on, `Ignored` otherwise.
- [x] 4.5 Fix `Scrollbars` orientation: construct/normalize internally to `VerticalRight` / `HorizontalBottom`, letting callers override only symbols/style.

## 5. Update call sites & prelude

- [x] 5.1 Update `prelude` / module re-exports for the renamed `Scrollbars` (+ `ScrollbarVisibility`).
- [x] 5.2 Update examples: `examples/components/scrollview.rs`, `wrapped_text.rs`, `table.rs` to the new API (`state`/`active`/`scrollbars`, `over_border` where relevant).
- [x] 5.3 Update the internal adopter `components/shortcut_info_modal.rs` to the new API.
- [~] 5.4 In `examples/components/table.rs`, use `scroll_to_visible` so the selected row follows the viewport. DEFERRED: the `scroll_to_visible(y, height)` state primitive + tests are delivered, but wiring it into the Table example needs a component-level "selected row → buffer y" mapping (see design Open Questions). Table example keeps external-drive selection + manual PageUp/Down for now.

## 6. Verification, docs & knowledge

- [x] 6.1 Full CI gate green: `cargo test --locked --all-features --workspace --lib --tests --examples`, `cargo clippy --all-targets --all-features --workspace -- -D warnings`, `cargo fmt --all --check`, `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items --all-features --workspace --examples`.
- [x] 6.2 Re-record `docs/tapes/scrollview.tape` (and any affected tapes) if the visible behavior/border look changed; verify frames via VHS `Screenshot`.
- [x] 6.3 Add a ScrollView component docs page (en + zh) or update existing references for the new API + `over_border`; document the content-buffer u16 ceiling and ScrollView-vs-VirtualList guidance.
- [x] 6.4 Update skill `references/components.md` ScrollView entry (props `state`/`active`/`scrollbars`, `over_border`, `is_at_bottom`, `scroll_to_visible`).
- [x] 6.5 Update `dev-notes/knowledge/runtime-architecture.md` (the ScrollView handler/nesting notes) and mark `scrollview-upstream-sync-pending` memory resolved after archive.
