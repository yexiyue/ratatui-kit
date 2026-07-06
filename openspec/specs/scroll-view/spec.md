# scroll-view Specification

## Purpose
TBD - created by archiving change scrollview-overhaul. Update Purpose after archive.
## Requirements
### Requirement: Block inner area is a single source of truth

When a `ScrollView` has a `block`, the region used for content layout, the visible viewport, offset clamping, scrollbar placement, and the blit target MUST all be derived from a single `block.inner()` computation. The scroll hook and the component's draw path MUST NOT compute the inner area two different ways, and the inner area MUST honor exactly which borders are present, any `Padding`, and any title rows.

#### Scenario: Full-border block reserves both side borders
- **WHEN** a `ScrollView` is given `block: Block::bordered()` on an outer area
- **THEN** the content/scrollbar/blit region equals `block.inner(outer)` (both left and right borders reserved; top and bottom borders reserved), with no column or row of content drawn onto a border cell

#### Scenario: Partial-border / padded block
- **WHEN** the block has only some borders, or `Padding`, or a title
- **THEN** the inner area shrinks by exactly those insets (matching `block.inner()`), and content never overlaps the reserved border/padding/title cells

### Requirement: Scrollbars-over-border is switchable

`Scrollbars` MUST expose an `over_border` toggle (default enabled) that selects where the scrollbar tracks are drawn relative to a block's border. Both axes MUST behave symmetrically.

#### Scenario: over_border enabled draws scrollbars on the border ring
- **WHEN** `over_border` is enabled and the `ScrollView` has a bordered block with scrollable content in both axes
- **THEN** the vertical scrollbar is drawn on the right border column and the horizontal scrollbar on the bottom border row, the content viewport occupies the full `block.inner()`, and where an axis shows no scrollbar its border remains intact

#### Scenario: over_border disabled insets the scrollbars
- **WHEN** `over_border` is disabled
- **THEN** the scrollbars are drawn inside `block.inner()` and the content viewport is `block.inner()` minus the shown scrollbar thickness on each occupied axis

### Requirement: Offset is clamped against the visible viewport

The scroll offset MUST be clamped so that the last row and last column of content are reachable. The maximum offset MUST be computed against the visible viewport (the inner area minus any scrollbar that steals space), not the raw render area.

#### Scenario: Last row reachable when a horizontal scrollbar is shown
- **WHEN** content is taller than the viewport and a horizontal scrollbar is shown (stealing one viewport row in inset mode)
- **THEN** scrolling to the bottom makes the final content row visible (the max vertical offset accounts for the row the horizontal scrollbar occupies)

### Requirement: Page size reflects the visible viewport

`ScrollViewState::page_size` MUST be set from the visible viewport after scrollbar resolution, so that `scroll_page_up`/`scroll_page_down` move by one page while preserving the intended one-row/column overlap.

#### Scenario: Page scroll preserves overlap with a scrollbar present
- **WHEN** a horizontal scrollbar is shown and the user pages down
- **THEN** the viewport advances by (visible-height − 1) rows, keeping one row of overlap between the old and new pages

### Requirement: is_at_bottom reports scroll position

`ScrollViewState` MUST provide `is_at_bottom()` that returns `true` before the first render (size unknown) and, after rendering, `true` exactly when the last content row is within the visible viewport.

#### Scenario: Reports bottom only when the final row is visible
- **WHEN** the viewport is scrolled so the last content row is visible
- **THEN** `is_at_bottom()` returns `true`; while any content below the viewport remains, it returns `false`

### Requirement: Scroll a target row into view

`ScrollViewState` MUST provide a way to scroll a target content row (given its y position and height) into the visible viewport, adjusting the offset only when the target is outside the current viewport.

#### Scenario: Target below the viewport scrolls into view
- **WHEN** a target row lies below the current viewport and the caller requests it be made visible
- **THEN** the offset advances just enough for the target row to be fully visible; a target already within the viewport does not change the offset

### Requirement: Nested ScrollView is safe

Rendering a `ScrollView` inside another `ScrollView` MUST NOT panic or corrupt the frame. The shared render scroll buffer MUST be saved and restored so each level blits its own content and the enclosing level's buffer survives.

#### Scenario: ScrollView inside ScrollView renders without panic
- **WHEN** a `ScrollView` is placed among the children of another `ScrollView`
- **THEN** both levels render their visible windows correctly and no panic occurs (no `take().unwrap()` on an absent buffer)

### Requirement: External state does not disable built-in interaction

Passing an external `state` handle MUST NOT disable built-in keyboard/mouse scrolling. Built-in interaction MUST be gated solely by `active`, so that a caller can both hold a state handle (to read offset / drive programmatic scroll) and keep built-in scrolling — matching the orthogonal `state`/`active` convention of the other selection components.

#### Scenario: State handle plus active keeps built-in scrolling
- **WHEN** a `ScrollView` is given an external `state` and `active` is true
- **THEN** arrow/PageUp/PageDown/wheel input still scrolls the view, and the external state reflects the updated offset

### Requirement: Event handling reports consumption

`ScrollViewState::handle_event` MUST report whether it acted on the event, and the built-in handler MUST return `Consumed` for input it scrolled on so the event does not silently propagate to sibling handlers.

#### Scenario: A handled scroll key is consumed
- **WHEN** a key that maps to a scroll action is dispatched to an active `ScrollView`
- **THEN** the view scrolls and the event is reported as consumed (not passed on to background handlers)

### Requirement: Scrollbar orientation is fixed

`Scrollbars` MAY let callers override scrollbar symbols and style, but the rendered orientation MUST be fixed to vertical-right and horizontal-bottom regardless of any orientation on a caller-provided `Scrollbar`, so layout math and rendering never disagree.

#### Scenario: Caller-provided orientation is normalized
- **WHEN** a caller supplies a `Scrollbar` configured with a non-right/non-bottom orientation
- **THEN** the vertical bar still renders on the right and the horizontal bar on the bottom, using the caller's symbols/style

### Requirement: Public API is consistent with sibling components

`ScrollView` MUST expose its external state handle as `state`, its interaction gate as `active` (default true), and its scrollbar configuration as `scrollbars` of type `Scrollbars`, matching the naming used by `Select`, `MultiSelect`, `TreeSelect`, `VirtualList`, and `Table`.

#### Scenario: Props use the shared naming
- **WHEN** an application writes a `ScrollView` in the `element!` macro
- **THEN** it uses `state:`, `active:`, and `scrollbars:` (type `Scrollbars`), with no `scroll_view_state`, `disabled`, or `ScrollBars` in the surface

