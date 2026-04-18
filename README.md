# fig2r

> Figma → pixel-perfect React + Tailwind, from your terminal. Built for AI coding agents.

[![npm](https://img.shields.io/npm/v/fig2r.svg)](https://www.npmjs.com/package/fig2r)
[![downloads](https://img.shields.io/npm/dm/fig2r.svg)](https://www.npmjs.com/package/fig2r)
[![license](https://img.shields.io/npm/l/fig2r.svg)](./LICENSE)

Point fig2r at a Figma node URL. It fetches the design, downloads every asset, and writes idiomatic React + Tailwind components you — or your agent — can drop into Next.js, Vite, or Remix.

The output is a **reference**, not a final component. Your agent then rewires it to match your project's design system, tokens, and components.

## Install

```bash
npm install -g fig2r
```

Set a Figma token (scope `file_content:read`):

```bash
fig2r auth figd_xxxxxxxxxxxx
# or: export FIGMA_TOKEN=figd_xxx
```

## Quick start

```bash
fig2r fetch "https://www.figma.com/design/FILE/Name?node-id=123-456" \
  --save ./components \
  --public-dir ./public
```

That's it. Open `./components/` — `.tsx` files ready to import.

## Commands

```bash
fig2r fetch <url> --save <dir>   # Figma → components + assets
fig2r fetch <url>                # stream IR JSON to stdout
fig2r convert <ir.json> -o <dir> # IR JSON → components (offline)
fig2r validate <ir.json>         # schema check
fig2r auth <token>               # save token to ~/.fig2r/config.toml
```

Useful flags: `--public-dir`, `--svg-mode {react-component|file|inline}`, `--naming {pascal|kebab}`, `--no-theme`. `convert` also accepts `--strict` (fail on any unsupported construct, for CI). See `fig2r <cmd> --help`.

## Agent skill (auto-linked)

On install, fig2r symlinks a bundled skill into any agent skill directory it finds:

- `~/.claude/skills/fig2r` — Claude Code
- `~/.agents/skills/fig2r` — Cursor, Codex, Aider, Cline

Your agent reads the skill and knows the whole workflow: fetch into `/tmp`, read the reference, adapt to the target project (shadcn, Radix, custom tokens, `next/font`, etc.), dedupe assets, verify.

Opt out with `FIG2R_SKIP_SKILL_INSTALL=1 npm install -g fig2r`. Uninstall with `rm ~/.claude/skills/fig2r ~/.agents/skills/fig2r`.

## What it handles

Flex + grid auto-layout, absolute positioning, z-index, padding/gap, per-side borders, rounded corners, solid/gradient/image fills, drop + inner shadows, blur, blend modes, opacity, rotation, flip, aspect ratio, variants, component properties, rich text, OpenType features, `next/font/google`, SVG paths, parallel asset download.

## Known limits

- `strokeGeometry` paths not rendered (use `fillGeometry`)
- Per-paint `blendMode` / `imageTransform` not honored
- Figma Variables flattened to resolved colors — mode switching not round-tripped
- Bullet/numbered text lists render as `<span>•</span>` + text, not semantic `<ul>/<ol>/<li>`
- `paragraphSpacing` emits a warning instead of wrapping paragraphs in `<p>` tags
- Gradient strokes fall back to the first stop's solid color; conic (`GRADIENT_ANGULAR`) approximated as linear
- Image fill filters (exposure/contrast/saturation) not applied in CSS
- React-only (no Vue / Svelte / SwiftUI)

## License

[MIT](./LICENSE) © Taradepan R
