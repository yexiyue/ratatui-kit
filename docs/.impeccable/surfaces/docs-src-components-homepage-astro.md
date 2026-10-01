---
version: 1
slug: "docs-src-components-homepage-astro"
primary_target: "docs/src/components/HomePage.astro"
related_targets: ["docs/src/components/HomePage.zh-CN.astro"]
---

Scope: docs homepage (EN `src/components/HomePage.astro` + zh-CN mirror), full-bleed splash under Starlight/nova. Visitor mode: Persuade.

Audience: Rust developers evaluating a TUI framework; many know React's model and want it in the terminal.
Job: understand and believe the model in one viewport; reach the right first doc.
Action: quick start CTA → `start/quick-start/`; github; component docs links; cargo add copy.
Proof/content: live HTML-rebuilt todo-app terminal mirroring `examples/apps/todo_app.rs` (same tasks, keybinds j/k/space/f/d/a/q, confirm modal, exit/restart); two real code panes with computed loc counts; six mechanism modules; input-layer depth diagram beside real `input_mutex.rs` code; `count += 1` reactive ripple; 14-component browser with real snippets and real GIF recordings; version/MIT from repo truth.
Constraints: bilingual parity; no fabricated social proof; `/ratatui-kit` subpath; both themes; reduced-motion respected; amber = live state only.
Chosen direction: 「终端即页面」 the site is a TUI app (seed 296d2b21, index 3, code-led).
Memorable moment: the hero terminal that runs itself, then answers your j/k.
Unresolved: none.
