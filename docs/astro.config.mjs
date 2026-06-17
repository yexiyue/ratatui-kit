// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import starlightThemeNova from 'starlight-theme-nova';

// https://astro.build/config
export default defineConfig({
	site: "https://yexiyue.github.io",
	base: "/ratatui-kit",
	integrations: [
		starlight({
			locales: {
				root: {
					label: '简体中文',
					lang: 'zh-CN'
				},
				// en: {
				//     label:'English',
				//     lang:'en'
				// }
			},
			plugins: [starlightThemeNova({
				nav: [
					{
						label: '学习', href: '/ratatui-kit/start/'
					},
					{
						label: '参考', href: '/ratatui-kit/components/'
					},
					{
						label: "示例", href: "/ratatui-kit/examples/"
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
					label: '学习：先跑起来',
					items: [
						'start',
						'start/installation',
						'start/quick-start',
						'start/mental-model',
					],
				},
				{
					label: '教程：从零到应用',
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
					label: '参考：核心模型',
					items: [
						'core/component-model',
						'core/declarative-syntax',
						'core/control-flow',
						'core/hooks',
						'core/input-layers',
						'core/state',
						'core/routing',
					],
				},
				{
					label: '参考：内置组件',
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
						'components/virtual-list',
						'components/virtual-multi-select',
					],
				},
				{
					label: '参考：高级扩展',
					items: [
						'advanced',
						'advanced/custom-hook',
						'advanced/custom-provider',
						'advanced/custom-widget',
					],
				},
				{
					label: '示例：源码路线图',
					items: [
						'examples',
					],
				},
				{
					label: '内部机制',
					collapsed: true,
					items: [
						'internals/render-loop',
					],
				},
			],
		}),
	],
});
