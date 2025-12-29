// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import starlightThemeNova from 'starlight-theme-nova';

// https://astro.build/config
export default defineConfig({
	base: "/ratatui-kit",
	integrations: [
		starlight({
			routeMiddleware: './src/routeData.ts',
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
						label: '文档', href: '/ratatui-kit/guides/intro/'
					},
					{
						label: "原理", href: "/ratatui-kit/principle/01-前言"
					}
				]
			})],
			title: 'Ratatui Kit',
			social: [{ icon: 'github', label: 'GitHub', href: 'https://github.com/yexiyue/ratatui-kit' }],
			sidebar: [
				{
					label: '指南',
					autogenerate: { directory: 'guides' },
				},
				{
					label: '示例',
					autogenerate: { directory: 'example' },
				},
				{
					label: '原理',
					autogenerate: { directory: 'principle' },
				},
			],
		}),
	],
});
