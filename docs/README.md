# Ratatui Kit Docs

这个目录是 ratatui-kit 的 Astro + Starlight 文档站。线上路径使用 GitHub Pages 项目站点：

```text
https://yexiyue.github.io/ratatui-kit/
```

## Commands

在 `docs/` 目录下运行：

| Command | Action |
| --- | --- |
| `pnpm install --frozen-lockfile` | 安装文档站依赖 |
| `pnpm dev` | 启动本地开发服务器 |
| `pnpm build` | 构建静态站点到 `dist/` |
| `pnpm preview` | 预览构建结果 |

## Content

- 文档页面放在 `src/content/docs/`。
- 可复现的 VHS 脚本放在 `tapes/`。
- 录制产物放在 `public/recordings/`。
- Mermaid 流程图通过 `src/components/Mermaid.astro` 显式引入。

新增教程或组件页时，优先保持这条链路：

```text
example -> docs/tapes/<name>.tape -> public/recordings/<name>.gif -> docs page
```
