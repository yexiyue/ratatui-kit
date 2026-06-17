# Ratatui Kit Docs

This directory contains the Astro + Starlight documentation site for ratatui-kit.
The production site is published as a GitHub Pages project site:

```text
https://yexiyue.github.io/ratatui-kit/
```

The default locale is English at the root path. Simplified Chinese content lives under:

```text
https://yexiyue.github.io/ratatui-kit/zh-cn/
```

See [README.zh-CN.md](README.zh-CN.md) for the Chinese version of this file.

## Commands

Run these commands from `docs/`:

| Command | Action |
| --- | --- |
| `pnpm install --frozen-lockfile` | Install documentation dependencies |
| `pnpm dev` | Start the local development server |
| `pnpm build` | Build the static site into `dist/` |
| `pnpm preview` | Preview the built site |

## Content

- English docs live in `src/content/docs/`.
- Simplified Chinese docs live in `src/content/docs/zh-cn/`.
- Reproducible VHS scripts live in `tapes/`.
- Recorded assets live in `public/recordings/`.
- Mermaid diagrams are imported explicitly through `src/components/Mermaid.astro`.

When adding a new tutorial or component page, keep this chain intact:

```text
example -> docs/tapes/<name>.tape -> public/recordings/<name>.gif -> docs page
```
