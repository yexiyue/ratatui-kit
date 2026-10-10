# Pointer interactions

Read when adding mouse actions, hover, dragging, or a mouse-driven modal. Source:
`crates/ratatui-kit/src/hooks/use_pointer.rs`, `components/button.rs`,
`input/mod.rs`; runnable composition: `examples/input/mouse.rs`.

## Entry and API choice

```rust
use ratatui_kit::{prelude::*, crossterm::event::MouseButton};
// In async main:
element!(App).fullscreen_with(RunOptions { mouse: true }).await?;
// Inline viewport:
element!(App).render_loop_with(terminal_options, RunOptions { mouse: true }).await?;
```

Mouse APIs are core, not `input`-feature gated. Capture defaults off. Pointer
registration without capture asserts in debug builds. Enabled modes are restored
on exit. Hover / track_move requests any-motion after the first relevant update,
and it stays enabled until exit. Terminal support determines whether hover works.

| Need | Prefer |
| --- | --- |
| Visible action on left Down | `Button` |
| Down + Up inside same component | `use_click` |
| Boolean hover feedback | `use_hover` |
| Left-button drag phases | `use_drag` |
| Event filtering / propagation control | `use_pointer` |

Button has no keyboard activation/focus. Add separate shortcuts calling the same
business action. Button props and `ButtonTheme` are listed in `components.md`.

## Signatures and event types

```rust
fn use_click<F>(&mut self, f: F) where F: FnMut(Click) + 'static;
fn use_hover(&mut self) -> bool;
fn use_drag<F>(&mut self, f: F) where F: FnMut(DragUpdate) + 'static;
fn use_pointer<F>(&mut self, options: PointerOptions, f: F)
    where F: FnMut(PointerEvent) -> EventResult + 'static;
```

Hook callbacks need no Send/Sync. Prop callbacks such as Button.on_press retain
`Handler<'static, ()>`'s Send + Sync requirement. Keep hook order stable,
including disabled or externally controlled components. Do not call a fallback
`use_state` only inside `Option::unwrap_or_else`; initialize the local state every
frame, then choose `props.state.unwrap_or(local)`.

- `PointerOptions { buttons: MouseButtons, track_move: bool }`: default LEFT,
  no movement tracking. Combine sets with `.union`; Scroll ignores the button set.
- `PointerEvent { kind, position, modifiers, area }`: no separate button field.
  Button identity is in `PointerKind::Down/Up/Drag(MouseButton)` or
  `Click { button, count }`. Other kinds: Move, Enter, Leave, ScrollUp/Down/Left/Right.
- `Click { count, button, position, modifiers }`: use_click accepts all buttons;
  300ms nearby consecutive clicks count 1, 2, 3 (capped). Each release invokes the
  callback immediately; double-click logic must account for the preceding count 1.
- `DragUpdate { phase, button, from, to, modifiers }`: use_drag is left-only.
  Phases Start/Move/End; `from` is the previous event, not the gesture origin.

Positions and areas use absolute terminal-cell coordinates. For a local width,
apply deltas rather than assigning `to.x` (margins/offsets otherwise introduce jumps):

```rust
hooks.use_drag(move |d| {
    if d.phase == DragPhase::Move {
        let delta = i32::from(d.to.x) - i32::from(d.from.x);
        split.set((i32::from(split.get()) + delta).clamp(10, 60) as u16);
    }
});
```

## Delivery rules that affect composition

- Consuming Down in a hit-tested pointer handler captures that button. Subsequent
  Drag/Up go exclusively to the captor before Global/layer handlers, even outside
  its region. Up releases; unmount or inactive layer invalidates capture.
- Otherwise Global handlers run first. Active-layer z order precedes priority.
  Equal-priority mouse delivery uses reverse draw order (child/later sibling
  first); keyboard delivery keeps registration order.
- `Consumed` stops propagation; `Ignored` continues it. use_click consumes
  press/release. For custom pass-through use_pointer and return Ignored.
- use_pointer synthesizes Click after Up if it can pair the press. Consuming Down
  ensures capture preserves the release delivery; release outside does not click.
- Hover respects active input layers and updates even if a Global consumer stops
  Moved delivery. Unchanged hover + unconsumed Moved can skip redraw; write State
  for visible changes rather than relying on every move redrawing.
- Hit testing uses the last draw's frame-stamped rectangle. Visual clipping is
  not intersected with it. Nested ScrollViews do not hand wheel events outward
  at their limits. Pointer input does not establish keyboard focus.

## Modal dismissal

Each frame declare `let layer = hooks.use_input_layer(open, true)`, register the
closer with `EventScope::Layer(layer)`, and pass `Modal(layer: Some(layer))`.
Use Low priority for a scrim closer with `EventOptions { hit_test: false }`.
Buttons inside consume Down first; unhandled scrim/panel blank clicks close.
Disabled buttons pass through and can reach the closer. Add a panel pointer
handler consuming Down if panel blanks should keep the modal open. Mount it as
its own component so its hit area matches the panel; do not attach the handler
to the whole root screen. Keep layer handles within the current frame.
