# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Stack

Existing site: Astro 6 + Starlight + `starlight-theme-nova` in `docs/` (pnpm), deployed to GitHub Pages at `https://yexiyue.github.io/ratatui-kit/` (subpath base). Homepage is a custom splash component (`docs/src/components/HomePage.astro` + `HomePage.zh-CN.astro`); docs chrome is themed via `docs/src/styles/brand.css`.

## Users

Primary: Rust developers building terminal apps with Ratatui who are tired of hand-written render loops, manual event fan-out, and per-app state plumbing. Many know React-style component/hooks models from web work and want that mental model in the terminal. Secondary: maintainers/evaluators comparing TUI frameworks (e.g. against Go's Bubble Tea).

## Product Purpose

Ratatui Kit is a component framework on top of Ratatui + Tokio: a JSX-style `element!` proc-macro, hooks (`use_state` / `use_future` / `use_effect` …), input layers as an event-ownership mutex, routing, atoms for global state, theming via a shared `Palette`, and ~15 business-neutral built-in components. The website's job: make a Rust developer understand and believe that model within one viewport, then route them into the docs with the right first page.

## Positioning

"The React model you already know, in the terminal" — declarative `element!` syntax plus hooks and input layers, compiling to Ratatui widgets. Raw Ratatui gives you widgets but no component/state/event model; Bubble Tea (Go) has messages but not this hooks + layered-input model in Rust.

## Operating Context

Bilingual docs (English root + `zh-cn` locale, both maintained in parity). Examples are runnable (`cargo run --example counter|router|store|modal|input|scrollview|todo_app …`) and recorded as GIFs in `docs/public/recordings/`. Crate version is read from `crates/ratatui-kit/Cargo.toml` at build time. Open source on GitHub (yexiyue/ratatui-kit).

## Capabilities and Constraints

- Site must work under the `/ratatui-kit` subpath (asset/recording URLs derive from `BASE_URL`).
- Light and dark themes are both first-class; `prefers-reduced-motion` respected.
- No fabricated social proof: no invented testimonials, stars, or download numbers. Real assets only: code from actual examples, real GIF recordings, version from Cargo.toml.
- The `textarea` feature is disabled (ratatui 0.30 compatibility) — do not advertise it.

## Brand Commitments

- Logo asset `docs/src/assets/logo.svg`: teal `#064A4F`, amber `#FCA91C`, cream `#FCF8EF`.
- User-confirmed (2026-10-01): keep the teal+amber duo as the brand anchor but simplify hard around it — neutral deep background, single-accent discipline, no third hue competing. Full-site redesign scope approved; hero gets an HTML/CSS-rebuilt terminal demo (GIFs stay for the component showcase).

## Evidence on Hand

- Real code snippets per component (mirrored from docs pages) in `HomePage.astro`.
- 20+ real GIF recordings (`docs/public/recordings/*.gif`).
- Real install/version info (Cargo.toml + `__RK_VERSION__`).
- Absent (must not be fabricated): testimonials, user counts, benchmarks, customer logos.

## Product Principles

1. Prove with real code and real recordings — the demo is the argument, not adjectives.
2. Simple beats clever — restraint reads as quality to this audience.
3. EN/zh-CN parity is a commitment, not an afterthought.
4. The terminal is the product: terminal demos must be the crispest, highest-contrast elements on the page.

## Accessibility & Inclusion

Both color schemes must stay legible (WCAG AA text contrast); motion is progressive enhancement behind `prefers-reduced-motion`.
