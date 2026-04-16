# fig2r

One-shot Figma → pixel-perfect React + Tailwind, built for AI coding agents (Claude Code, Cursor, Codex, etc.).

Point your agent at a Figma node URL. fig2r fetches the design, downloads every asset, and writes React components you can drop into a Next.js / Vite / Remix project. The output is an adaptable reference — your agent can then rewire it to your design system, tokens, and components.

## Install

```bash
npm install -g fig2r
# or: cargo install --path .
```

First run needs a Figma token (`file_content:read` scope):

```bash
fig2r auth figd_xxx   # stored in ~/.fig2r/config.toml, chmod 0600
# or: export FIGMA_TOKEN=figd_xxx
# or: --token figd_xxx
```

## Usage

```bash
# Fetch + generate components + download assets in one step
fig2r fetch "https://www.figma.com/design/FILE/Name?node-id=123-456" --save ./components

# Stream IR JSON to stdout (pipe into an agent or another tool)
fig2r fetch "https://..." > design.ir.json

# Convert an existing IR JSON to components
fig2r convert design.ir.json -o ./components
cat design.ir.json | fig2r convert -o ./components

# Validate IR without writing files
fig2r validate design.ir.json
```

Common flags (`fig2r fetch --help` for all):

| Flag | Purpose |
|---|---|
| `--save <dir>` | Write component files instead of printing IR |
| `--public-dir <dir>` | Route image assets to `<dir>/assets/` (Next.js `/public`) |
| `--svg-mode` | `react-component` (default), `file`, or `inline` |
| `--naming` | `pascal` or `kebab` component names |
| `--no-theme` | Skip theme token extraction |
| `--strict` | Fail on any unsupported construct (CI mode) |

## Agent workflow

The primary use case is an LLM-driven loop:

1. Agent gets a Figma URL from the user.
2. Agent runs `fig2r fetch <url> --save <tmp>`.
3. Agent reads the generated React + Tailwind as a reference.
4. Agent adapts it to the target project's components, tokens, and conventions.

The emitted code is deliberately concrete (arbitrary Tailwind values like `px-[13px]`, literal hex) so the agent sees the exact design intent and can decide what to keep vs. replace with tokens.

## Output

```
components/
  Container/
    Container.tsx       # React component
    index.ts            # re-export
  icons/                # SVG vectors as React components
  theme.ts              # design tokens (optional)
public/assets/          # PNG/JPG/SVG image fills
```

Fonts: `next/font/google` imports are emitted with `variable: '--font-xxx'` and wired to descendants via a `display: contents` wrapper so the mangled family name resolves correctly.

## Architecture

Thin `main.rs` parses CLI args and dispatches. Pipeline:

**Figma REST API → IR JSON → codegen tree → emitted files**

| Module | Role |
|---|---|
| `cli` | clap derive subcommands |
| `figma` | API client, URL parser, token config, node → IR transform |
| `ir` | IR schema (`serde`) + validation |
| `codegen` | IR → component tree, assets, theme tokens, variants |
| `tailwind` | IR → Tailwind class strings |
| `emit` | file writer + formatter |

IR JSON is stable across runs and versioned, so agents can cache it or pipe it between tools.

## What fig2r handles

Layout (flex + grid), auto-layout, absolute positioning, z-index, padding/gap, per-side borders, rounded corners (including iOS squircle fallback), solid/gradient/image fills, drop + inner shadows, blur, blend modes, opacity, rotation, flip, aspect ratio, fixed/fill/hug sizing, min/max constraints, variants, component properties, images (cover/contain/crop), SVG paths, rich text spans, font-family + weight + line-height + letter-spacing + decoration + list bullets + OpenType features.

## Known limits

- Vector paths from `strokeGeometry` aren't rendered (use `fillGeometry` only).
- Per-paint `blendMode` and `imageTransform` (crop pan position) aren't honored — node-level blend + `scaleMode` cover the common cases.
- Figma Variables are flattened to their resolved color; mode switching isn't round-tripped.
- `strokesIncludedInLayout` isn't translated to box-sizing offsets.

## Requirements

- Node 18+ (for the npm shim)
- Rust edition 2024 if building from source

## License

MIT
