---
kind: frontend_style
name: Native Dark Glass UI with CSS Custom Properties Design Tokens
category: frontend_style
scope:
    - '**'
source_files:
    - src/styles.css
    - index.html
    - vite.config.ts
    - package.json
---

## Approach

The FreeSpace desktop app uses a **plain CSS** styling approach (no framework, no preprocessor, no component library). All styles live in a single stylesheet `src/styles.css` (1554 lines) and are loaded directly from the root `index.html` via `<link rel="stylesheet" href="/src/styles.css">`. The build pipeline is Vite + TypeScript (`vite.config.ts`), which compiles `.ts` but passes `.css` through unchanged.

There is no Tailwind, PostCSS plugin, CSS-in-JS, or design-token compiler — tokens are expressed as native CSS custom properties on `:root`.

## Design Tokens (`:root`)

All visual values are centralized under `:root` at the top of `src/styles.css`:

- **Colors**: semantic backgrounds (`--bg-primary`, `--bg-secondary`, `--bg-card`, `--bg-card-hover`, `--bg-card-border`), accent palette (`--accent-blue`, `--accent-green`, `--accent-amber`, `--accent-red`, `--accent-purple`) plus their dimmed variants (`*-dim` for selected states), text levels (`--text-primary`, `--text-secondary`, `--text-muted`), and ring colors (`--ring-used`, `--ring-free`).
- **Spacing / Radius**: `--radius-sm/md/lg/xl` (8–20px).
- **Transitions**: `--transition-fast/normal/slow` using `cubic-bezier(0.4, 0, 0.2, 1)`.
- **Typography**: system font stack targeting macOS (`-apple-system, BlinkMacSystemFont, "SF Pro Display", "SF Pro Text", "Helvetica Neue", system-ui, sans-serif`), base size `13px`, line-height `1.5`, antialiased.

These variables are consumed throughout the stylesheet; there is no secondary token file.

## Visual Style

- **Dark theme**: background `#0d0f14` with subtle radial gradients (blue/purple) behind the app shell.
- **Glass morphism**: reusable `.glass-card` class applies `backdrop-filter: blur(20px)` + semi-transparent background + thin border, used for disk overview, results sections, toasts, and dialog overlays.
- **Tauri-native titlebar**: `#titlebar` uses `-webkit-app-region: drag` so the OS window chrome integrates with the custom header; interactive buttons inside use `-webkit-app-region: no-drag`.
- **Animations**: keyframes defined inline (`slideUp`, `fadeIn`, `spin`, `pulse`, `toastIn/Out`, `itemSlideIn`, `treeSlideDown`) with staggered delays applied via `nth-child` selectors up to item 13.
- **Custom scrollbar**: only `::-webkit-scrollbar` is styled (macOS/WebKit); Firefox/Edge get defaults.

## Architecture & Conventions

- **Single-file stylesheet**: all CSS is in `src/styles.css`; there are no per-component style files, SCSS, or CSS modules.
- **BEM-like naming**: classes follow a flat BEM-ish convention (e.g., `.results-section`, `.section-header`, `.cleanup-item`, `.item-checkbox`, `.action-bar`, `.toast-container`). No nesting beyond what's needed for state modifiers (`.selected`, `.hidden`, `.review`, `.checked`, `.indeterminate`).
- **State-driven visuals**: modifier classes toggle appearance (e.g., `.results-section.review` re-themes blue accents to amber for the "Needs Review" section; `.item-checkbox.checked` vs `.indeterminate` show checkmark vs dash).
- **No responsive breakpoints**: the layout is fixed-width desktop-only (the app targets macOS desktop windows). There are no `@media` queries in the stylesheet.
- **Icons**: emoji characters are used inline in HTML (`🧹`, `🔍`, `✅`, `⚠️`) rather than icon fonts or SVG sprites.
- **SVG charts**: the disk usage ring is an inline `<svg>` in `index.html` whose stroke/dashoffset is animated via CSS.

## Enforced Rules

- The stylesheet is imported as a plain CSS link in `index.html` — no bundler transformation is configured for it.
- `vite.config.ts` contains no CSS-related plugins; Vite's default CSS handling is used.
- `package.json` has no CSS-related dependencies (no tailwind, sass, postcss, autoprefixer, cssnano).
- Typography is locked to the macOS system font stack; no web fonts are loaded.
- Tauri-specific interop is done via `-webkit-app-region` attributes/classes on the titlebar and its interactive children.

## Key Files

- `src/styles.css` — complete stylesheet (design tokens, reset, components, animations).
- `index.html` — static shell that references `/src/styles.css` and hosts the DOM structure.
- `vite.config.ts` — Vite config (no CSS plugins).
- `package.json` — confirms absence of any CSS tooling.