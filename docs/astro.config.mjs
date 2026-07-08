// @ts-check
import { readFileSync } from 'node:fs';
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import starlightThemeNova from 'starlight-theme-nova';

// Single source of truth: read the main crate version from Cargo.toml at build startup
// and expose it to MDX/Astro through __RK_VERSION__ (see src/consts.ts).
const cargoToml = readFileSync(
	new URL('../crates/ratatui-kit/Cargo.toml', import.meta.url),
	'utf-8',
);
const RK_VERSION = cargoToml.match(/^version\s*=\s*"([^"]+)"/m)?.[1] ?? '0.0.0';

// https://astro.build/config
export default defineConfig({
	site: "https://yexiyue.github.io",
	base: "/ratatui-kit",
	vite: {
		define: {
			__RK_VERSION__: JSON.stringify(RK_VERSION),
		},
	},
	integrations: [
		starlight({
			defaultLocale: 'root',
			locales: {
				root: {
					label: 'English',
					lang: 'en',
				},
				'zh-cn': {
					label: '简体中文',
					lang: 'zh-CN',
				},
			},
			plugins: [starlightThemeNova({
				nav: [
					{
						label: { root: 'Learn', 'zh-CN': '学习' },
						href: { root: '/ratatui-kit/start/', 'zh-CN': '/ratatui-kit/zh-cn/start/' },
					},
					{
						label: { root: 'Reference', 'zh-CN': '参考' },
						href: { root: '/ratatui-kit/components/', 'zh-CN': '/ratatui-kit/zh-cn/components/' },
					},
					{
						label: { root: 'Examples', 'zh-CN': '示例' },
						href: { root: "/ratatui-kit/examples/", 'zh-CN': "/ratatui-kit/zh-cn/examples/" },
					}
				]
			})],
			title: 'Ratatui Kit',
			logo: {
				src: './src/assets/logo.svg',
			},
			favicon: '/favicon.svg',
			customCss: ['./src/styles/brand.css'],
			social: [{ icon: 'github', label: 'GitHub', href: 'https://github.com/yexiyue/ratatui-kit' }],
			sidebar: [
				{
					label: 'Learn: get running',
					translations: { 'zh-CN': '学习：先跑起来' },
					items: [
						'start',
						'start/installation',
						'start/quick-start',
						'start/mental-model',
						'start/ai-skill',
					],
				},
				{
					label: 'Tutorials: from zero to app',
					translations: { 'zh-CN': '教程：从零到应用' },
					items: [
						'tutorials/counter',
						'tutorials/async-state',
						'tutorials/atom-state',
						'tutorials/input-mutex',
						'tutorials/router',
						'apps/todo-app',
					],
				},
				{
					label: 'Reference: core model',
					translations: { 'zh-CN': '参考：核心模型' },
					items: [
						'core/component-model',
						'core/declarative-syntax',
						'core/control-flow',
						'core/hooks',
						'core/input-layers',
						'core/state',
						'core/theming',
						'core/routing',
					],
				},
				{
					label: 'Reference: built-in components',
					translations: { 'zh-CN': '参考：内置组件' },
					items: [
						'components',
						'components/layout-primitives',
						'components/scroll-view',
						'components/wrapped-text',
						'components/input',
						'components/search-input',
						'components/modal',
						'components/confirm-modal',
						'components/alert-modal',
						'components/shortcut-info-modal',
						'components/select',
						'components/multi-select',
						'components/tree-select',
						'components/table',
						'components/virtual-list',
						'components/virtual-multi-select',
					],
				},
				{
					label: 'Reference: advanced extensions',
					translations: { 'zh-CN': '参考：高级扩展' },
					items: [
						'advanced',
						'advanced/custom-hook',
						'advanced/custom-provider',
						'advanced/custom-widget',
					],
				},
				{
					label: 'Examples: source roadmap',
					translations: { 'zh-CN': '示例：源码路线图' },
					items: [
						'examples',
					],
				},
				{
					label: 'Internals',
					translations: { 'zh-CN': '内部机制' },
					collapsed: true,
					items: [
						'internals/render-loop',
					],
				},
			],
		}),
	],
});
