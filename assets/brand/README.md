# Ratatui Kit brand assets

终端界面，从容构建。 / Terminal UI. Composed.

## 设计方向

保留既有 logo 的深青底色、象牙白终端符号与琥珀色角标。参考 Ratatui 官方的像素鼠、等宽字体和硬边终端图形，用像素构建者、字符框线与真实终端录屏表达框架气质。Ratatui 提供绘制与 widgets；Ratatui Kit 在其上封装组件、Hooks、状态、输入层与布局结构。

设计参数：布局变化 6/10，动效 2/10，信息密度 3/10。官网使用英文与中文同构布局，支持明暗主题与移动端。录屏默认显示静态封面，由用户主动播放；切换示例时停止上一段录屏。无需自动动画，也不依赖滚动特效。

## 角色方向：终端构建者

![像素终端构建者](mascot-assembler.png)

角色是一只轮廓清晰、长吻、小眼睛的像素鼠，穿深青工装，胸前保留终端符号，用一个终端框架承载组件块与光标。姿态强调构建、组合与检查，服务于开发工具的表达。

以 `mascot-assembler.png` 为角色源文件。制作新姿态时保留像素轮廓、长吻、小眼睛、阶梯状细尾、深青工装、象牙白肢体和琥珀色细节。保持透明背景、完整角色和组件框线。避免幼态大头、圆润玩偶、黏土质感及夸张卖萌。

[参考 Ratatui 官方网站](https://ratatui.rs/)（2026-10-10 实际检查）。借鉴其终端文化与生态关系；角色为原创延展，主标志继续使用 Ratatui Kit 原始 SVG。

## 颜色与字体

| 用途 | 色值 |
| --- | --- |
| 原始标志深青 | `#064A4F` |
| 原始标志琥珀 | `#FCA91C` |
| 象牙白、亮色页面背景 | `#FCF8EF` |
| 深色页面背景 | `#081518` |
| 亮色页面强调文字 | `#905300` |

官网主标题、代码与编号采用 JetBrains Mono Variable；正文与章节标题采用 Space Grotesk Variable。中文由系统无衬线字体回退。矢量字标使用系统 Arial，便于独立打开；正式排版以官网字体为准。

## 资产清单

- `mascot-assembler.png`：像素终端构建者的透明原图。
- `../../docs/public/brand/mascot-assembler.webp`：保留透明度的无损网页角色图。
- `explorations/`：早期材质图与已撤下的软萌小鼠，仅保留作探索记录。
- `readme-cover.png`：像素构建者品牌封面原图，明确标注 Built on Ratatui。
- `../../docs/public/brand/hero.webp`：品牌封面的优化版网页素材。
- `../../docs/public/social-preview.png`：1200 × 630 社交预览及 README 封面。
- `../../docs/public/brand/logo.svg`：与既有 logo 完全相同的矢量标志。
- `../../docs/public/brand/wordmark-light.svg`、`wordmark-dark.svg`：矢量横向字标，嵌入原始标志。
- `../../docs/public/brand/*-poster.webp`：真实 VHS 录屏应用运行后的静态画面（多数取第 6 秒，table 取第 8.4 秒）。

主 logo 的唯一来源是 `docs/src/assets/logo.svg`。不要从生成图中描摹或提取新 logo。AI 角色与造型细节属于宣传插画；应用图标、导航、favicon 均使用原始标志。

## 网站调研

调研日期：2026-10-10。以下是设计观察与本项目的取舍，并非对这些网站的功能评价。

- [Linear](https://linear.app/)：清晰的产品主张与产品画面支撑分段叙事。借鉴产品证据先行的结构，用真实终端录屏展示能力。
- [Zed](https://zed.dev/)：当前 Zed Industries 首页采用浅色编辑式排版与有辨识度的产品物体。借鉴留白、材质与强识别色，延续本项目的深青和琥珀。
- [Raycast](https://www.raycast.com/)：开发工具的品牌表达与功能介绍结合。保留明确的开始入口，避免把装饰置于安装与示例之前。
- [Ratatui](https://ratatui.rs/)：终端生态的文档入口与真实使用路径。本站继续保留快速开始、组件参考与示例导航。

## 重新导出

在仓库根目录运行 `node docs/scripts/brand-assets.mjs`。脚本从已选定的主视觉原图、原始 SVG 和真实 GIF 生成网页资产，不重新调用图像生成。替换源图或录屏后，重新导出并检查所有画面。

## 维护

中英文首页统一在 `docs/src/components/HomePage.astro` 修改，中文包装组件只传入 locale。教程链接、Cargo 示例名称与录屏文件名分别维护，不应假定三者相同。新增首页示例必须提供真实录屏与静态封面，验证键盘 Tab/方向键、播放与暂停、390px/320px 手机布局、明暗主题，并运行 `cd docs && pnpm build`。
