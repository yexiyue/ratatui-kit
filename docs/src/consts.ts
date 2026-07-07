// Single source of truth: astro.config.mjs reads the crate version from Cargo.toml
// at build startup and injects it through vite.define as __RK_VERSION__.
// Runtime code should not touch the filesystem during prerendering.
declare const __RK_VERSION__: string;

/** The current published version of the main `ratatui-kit` crate, e.g. `x.y.z`. */
export const RK_VERSION: string =
	typeof __RK_VERSION__ === 'string' ? __RK_VERSION__ : '0.0.0';
