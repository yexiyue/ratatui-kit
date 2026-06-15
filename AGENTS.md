# Repository Guidelines

## Project Structure & Module Organization

This is a Rust 2024 Cargo workspace for a ratatui-based terminal UI framework. The main library lives in `packages/ratatui-kit/`; its modules are split by runtime concern: `components/`, `hooks/`, `render/`, `element/`, `terminal/`, `atom/`, and `context.rs`. Procedural macros live in `packages/ratatui-kit-macros/src/` and provide `element!`, `#[component]`, `#[derive(Props)]`, router, and layout helpers. The root crate `ratatui-kit-examples` only hosts runnable examples in `examples/`. Compile-fail/pass UI tests live under `packages/ratatui-kit/tests/ui/`. Design proposals and active specs are under `openspec/changes/`; project knowledge notes are under `dev-notes/knowledge/`.

## Build, Test, and Development Commands

- `cargo test --locked --all-features --workspace --lib --tests --examples`: run the full test matrix used by CI and lefthook.
- `cargo clippy --all-targets --all-features --workspace -- -D warnings`: lint all crates and fail on warnings.
- `cargo fmt --all --check`: verify formatting.
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items --all-features --workspace --examples`: build docs with CI warning policy.
- `cargo run --example counter`: run an example; other examples include `router`, `store`, `modal`, `input`, and `scrollview`.

## Coding Style & Naming Conventions

Use `rustfmt` with `tab_spaces = 4`. Keep code idiomatic Rust and edition 2024 compatible. Public API names use Rust conventions: `CamelCase` types/traits, `snake_case` functions/modules, and feature names such as `router`, `atom`, `input`, `tree`, and `full`. Comments, docs, and commit subjects in this repository are often Chinese; keep new text consistent with nearby files. When adding component props, prefer existing `Props`/`#[with_layout_style]` patterns; raw ratatui `Block<'static>` is allowed in props after the framework-level `Send + Sync` removal.

## Testing Guidelines

The project uses standard Cargo tests plus `trybuild` UI tests for macro behavior. Add runtime unit/integration tests near the affected crate when possible, and add macro pass/fail cases in `packages/ratatui-kit/tests/ui/pass/` or `fail/` with matching `.stderr` files. Always test feature-gated code with `--all-features`; the default feature set is intentionally empty.

## Commit & Pull Request Guidelines

Recent history follows Conventional Commit prefixes such as `feat:`, `feat!:`, `refactor:`, `test:`, `perf:`, `docs:`, and `chore:`. Use a short imperative subject, optionally scoped, for example `test(router): add path matching cases`. Pull requests should describe the change, list validation commands run, link relevant issues or OpenSpec changes, and include terminal screenshots only when UI behavior changes.

## Agent-Specific Instructions

Before development changes, read the relevant notes in `dev-notes/knowledge/` and any active `openspec/changes/*` files touching the area. Do not revive the disabled `textarea` feature without confirming dependency compatibility with ratatui 0.30.
