---
name: fig2r
description: >-
  Use when the user shares a Figma URL, says "implement this design",
  "match the figma", "convert figma to code", "build this from figma",
  updates an existing component to match a new Figma design, or any task
  involving Figma URLs that need to become production React code. Works with
  any AI coding agent (Claude Code, Cursor, Windsurf, Copilot, Codex, Aider,
  Cline) — CLI + file-based, no agent-specific tooling.
---

# fig2r — Figma Design Implementation

fig2r is a Rust CLI that turns a Figma URL into a pixel-perfect React+Tailwind
**reference component** in a temp directory. The reference is a **design
spec**, not production code. The agent reads the spec, then implements the
design in the target project using that project's own components, tokens,
and conventions.

## The Iron Law

**The reference goes in `/tmp`. It never lands in the project tree as-is.**

The project's code style, component library, design tokens, icon library,
and font pipeline all differ from fig2r's generic output. Copy-pasting the
reference ships class-soup that conflicts with everything else in the repo.

**Specifically, never:**
- Copy `/tmp/fig2r-output/ComponentName/ComponentName.tsx` into `src/`
- Keep raw hex values (`bg-[#3A422C]`) when the project has a matching token
- Ship `<img src="/assets/check.svg">` when the project already has
  `lucide-react` installed
- Import from fig2r's generated `icons/` folder when standard icons exist in
  the project's icon library
- Recreate `<Button>` / `<Card>` / `<Avatar>` primitives the project already
  has in `components/ui/`
- Use literal font family names when the project uses `next/font`
- Emit `h-full` / `self-stretch` / `min-h-0` on text leaves inside
  fixed-height parents (clips text)

## Workflow

### 1. Fetch

```bash
fig2r fetch "<figma-url>" --save /tmp/fig2r-output
```

Output:

```
/tmp/fig2r-output/
  ComponentName/ComponentName.tsx   # the reference spec
  ComponentName/index.ts            # re-export
  assets/                           # raster images, custom SVGs
  icons/                            # SVG React components
```

One-time setup:
```bash
npm install -g fig2r
fig2r auth "figd_..."   # token from Figma → Settings → Security
                        # scope: file_content:read
```

Alt: `export FIGMA_TOKEN=...`. Set `NO_UPDATE_NOTIFIER=1` in CI.

**If `fig2r` is not on PATH** (this skill is present but the CLI was never
installed globally — e.g. `command -v fig2r` returns nothing): use `npx`.
It works identically without a global install:

```bash
npx fig2r@latest fetch "<figma-url>" --save /tmp/fig2r-output
```

Every CLI command in this skill works the same way under npx — just prefix
with `npx fig2r@latest` instead of `fig2r`. Pin `@latest` so npx always
pulls current fixes. Still need `FIGMA_TOKEN` set (env var or `~/.fig2r/config.toml`).
Suggest the user run `npm install -g fig2r` if they'll use it more than once —
global install is faster per-run and auto-links this skill.

**Always run the latest version.** fig2r ships bug fixes and Figma-API
fidelity improvements regularly. On every `fig2r fetch` the CLI checks
npm once per 24h and prints to stderr when newer:

```
[fig2r] Update available: 0.1.1 → 0.2.0
        Run: npm install -g fig2r@latest
```

If that banner appears, **update before continuing**:

```bash
npm install -g fig2r@latest
```

Then re-run the fetch. The older version may have bugs already fixed
upstream — running it wastes the round-trip. Silence the banner in CI
with `NO_UPDATE_NOTIFIER=1` (don't silence it locally).

### 2. Read the reference

Read `/tmp/fig2r-output/ComponentName/ComponentName.tsx`. It encodes:
exact layout, colors, typography, sizing, borders, effects, positioning.
Treat every value as truth until a matching project token is found.

**Do not copy this file into the project.** Read it to understand the design.

### 3. Survey the project

Before writing any code, detect the stack. See
[references/project-adaptation.md](references/project-adaptation.md) §1 for
exact commands. At minimum, identify:

- Component library (shadcn / Radix / MUI / Chakra / Mantine / custom)
- Styling (Tailwind v3 / v4 / CSS Modules / styled-components / emotion)
- Icon library (lucide-react / heroicons / phosphor / react-icons)
- `cn()` utility import path
- Path alias (`@/`, `~/`, relative)
- Design tokens (Tailwind `theme.extend` / v4 `@theme` / CSS variables)
- Font setup (next/font with `variable:` / Google Fonts / @font-face)

Skipping this step = generic output that doesn't match the codebase.

### 4. Adapt and implement

Using the reference as the spec, edit files in the target project:

- **Reuse existing components** — if the project has `<Button>`, use it.
  Match variant by comparing colors/padding/border against its variant
  definition.
- **Map colors to tokens** — replace every `bg-[#xxx]` with the matching
  token (`bg-primary`, `bg-brand-default`, `bg-accent`). Raw hex is a
  fallback, not a default.
- **Map spacing/radii/font-size to tokens** — `rounded-[8px]` → `rounded-lg`
  if the project defines `lg` as 8px. Same for `p-*`, `text-*`, `gap-*`.
- **Replace icon SVGs with library components** — `<img src="/assets/check.svg">`
  → `<Check className="w-5 h-5" />`. Only keep SVG files for custom
  illustrations with no library match.
- **Match fonts via the project's pipeline** — with next/font, use
  `font-[var(--font-xxx)]`, never literal family names.
- **Match the exact design** — don't round `21px` to `20px` to "feel
  cleaner". Pixel-perfect means pixel-perfect.
- **Follow project patterns** — forwardRef, CVA variants, props interfaces,
  file layout, export style.
- **Deduplicate assets before copying** — fig2r writes every image to
  `/tmp/fig2r-output/assets/`. Before copying into `public/assets/`, check
  if the same asset already exists in the project:
  ```bash
  # Hash-compare: same bytes under a different name
  for f in /tmp/fig2r-output/assets/*; do
    h=$(shasum -a 256 "$f" | cut -d' ' -f1)
    match=$(find public -type f -exec shasum -a 256 {} + 2>/dev/null | grep "^$h " | head -1)
    [ -n "$match" ] && echo "DUPE: $f -> ${match##* }"
  done
  ```
  When a match exists, reference the existing path; do not copy the new file.
  Same-name-different-bytes means the design changed — rename (e.g.,
  `hero@v2.png`) rather than overwriting.
- **Handle failed downloads** — check stderr for
  `[SUMMARY] failed_downloads=N`. `N > 0` means some assets referenced in
  the `.tsx` were NOT written — shipping as-is gives broken `<img>` at
  runtime. Options in order of preference:
  1. **Re-run `fig2r fetch`** — most failures are transient (5xx from
     Figma S3).
  2. **Use the Figma MCP** — if installed (tool
     `mcp__plugin_figma_figma__get_screenshot` / `get_design_context`),
     fetch the missing image directly from the Figma node. Save into the
     project's public dir.
  3. **Use any other image-fetch tool** available (web fetch on the Figma
     image URL in `asset.url`, screenshot tools, etc.).
  4. **Ask the user** for the missing asset if automated recovery fails.
  
  Never leave the component referencing a file that isn't on disk.
- **Add interactivity** — fig2r output is static. Wire onClick, state,
  forms, routing, data fetching using the project's existing patterns.

Full mapping tables (shadcn/Radix/MUI/Chakra/Mantine, icons, tokens, fonts,
Tailwind v4, variants, dark mode) in
[references/project-adaptation.md](references/project-adaptation.md).

### 5. Verify before declaring done

- Every color class is a token (or justified hex)
- Every icon uses the project's icon library (or justified custom SVG)
- Every size/radius/spacing uses a token where one exists
- No primitives were recreated — existing components are used
- Font families route through the project's font pipeline
- Assets live under the project's public dir, not `/tmp`
- `failed_downloads=0` in the fetch summary (or every missing asset was
  recovered via MCP / re-fetch / user)
- No duplicate asset copied — existing project assets reused by path
- Component compiles (`tsc --noEmit` or equivalent)
- Static analysis passes (eslint / biome / whatever the project uses)

## Red flags — STOP

Any of these means you're about to violate the Iron Law:

- "I'll just copy the file over and tweak it" — NO. Read spec, rewrite.
- "The raw hex is fine for now, I'll refactor later" — NO. Map tokens on
  first pass; "later" doesn't come.
- "Fig2r's class list is longer than the project's, I'll trust it" — NO.
  Long class lists are an artifact of generic output; map to tokens.
- "I'll use the SVG file instead of looking up the icon" — NO. Five seconds
  to check the icon library saves a `public/assets/` bloat.
- "The project has `<Button>` but fig2r's div looks different, I'll keep
  the div" — NO. Match variants. If no variant matches, add a new one to
  `<Button>`; don't bypass it.
- "I'll skip the project survey, this one's obvious" — NO. Run the
  detection commands.
- "Download failed but I'll leave the `<img>` tag — user will notice the
  404" — NO. Re-run, use Figma MCP, or ask. Never ship broken asset refs.
- "I'll just copy all assets from /tmp into public/" — NO. Dedupe first.
  Existing project assets win.
- "Update banner printed but the current version works" — NO. Update
  first, re-run. Bugs in the old version may silently corrupt output.

## Rationalization → reality

| Thought | Reality |
|---|---|
| "Reference is close enough to production" | Reference is tokens-naive, library-naive, pattern-naive. Always adapt. |
| "The project is small, tokens don't matter" | Tokens compound; starting raw-hex makes every future design drift. |
| "I don't know the project well enough to adapt" | Run the survey commands in references/project-adaptation.md §1. Takes <2 min. |
| "Library component doesn't match the design exactly" | Edit the variant, or add a new one. Don't bypass the component. |
| "Icon lookup is tedious" | Fig2r emits filenames that match library names ~90% of the time. |
| "Agent can't tell if a hex matches a token" | Grep the config for the hex. If within 2-3 units, use the token. |
| "The user didn't ask for token mapping" | Token mapping is table stakes for production React code. |
| "failed_downloads warning is informational" | It means the `.tsx` points at files that don't exist. Always recover. |
| "Copying all assets is safer than deduping" | Safer = larger bundle, duplicate S3 bytes, and stale references when design updates. Dedupe. |
| "Update banner is informational" | It means a bug you'd hit may already be fixed. Update first, always. |

## CLI reference

```bash
fig2r fetch "<url>" --save <dir>        # fetch + generate
fig2r fetch "<url>"                     # IR JSON → stdout (pipe to convert)

# Useful flags on fetch:
--public-dir ./public                   # assets → ./public/assets/
--icon-library lucide                   # import hints (none|lucide|phosphor|heroicons|react-icons)
--responsive                            # root: w-full max-w-[Npx]
--naming kebab                          # kebab-case files (default: pascal)
--svg-mode file                         # svg handling (react-component|file|inline)
--no-theme                              # skip theme token extraction
--no-index                              # skip index.ts re-exports
--flat                                  # no per-component subdirectories
--cn-import "@/lib/utils"               # cn() import path
--quiet                                 # suppress non-error logs

fig2r auth "<token>"                    # saves ~/.fig2r/config.toml
fig2r convert <ir.json> -o <dir>        # convert pre-built IR
fig2r convert <ir.json> -o <dir> --strict   # fail on ambiguous nodes (CI)
fig2r validate <ir.json>                # well-formedness
```

`--strict` is on `convert`, not `fetch`. For CI gating:
`fig2r fetch <url> | fig2r convert -o <dir> --strict`.

## Common patterns

```bash
# New component — use --icon-library + --cn-import matching project
fig2r fetch "<url>" --save /tmp/fig2r-output \
  --icon-library lucide --cn-import "@/lib/utils"

# Next.js — route assets into public/
fig2r fetch "<url>" --save /tmp/fig2r-output --public-dir ./public

# Multiple sections of a page — fetch each node separately
fig2r fetch "<hero-url>"   --save /tmp/fig2r-hero
fig2r fetch "<footer-url>" --save /tmp/fig2r-footer
# adapt + compose in page.tsx

# Updating existing component — fetch, diff visually, edit in place
fig2r fetch "<new-design-url>" --save /tmp/fig2r-output
# preserve existing logic, state, handlers, tests; update only classes/structure
```

## What fig2r handles

Layout (flex, gap, padding, alignment, overflow, wrap), sizing (fixed px,
flex-1, min/max, shrink-0), absolute positioning, rotation, colors (hex,
hex+alpha, linear/radial gradients), typography (size, weight, family,
line-height, letter-spacing, alignment, italic, truncation, max-lines),
borders (width incl. fractional, per-side, color, dashed, per-corner
radius), effects (drop-shadow, inner-shadow, blur, backdrop-blur), opacity
(node and text level), images (PNG @2x, SVG vectors), icons (vector frames
as single SVG), semantic HTML (button/header/nav/footer/section/aside/hr
from layer names), component variants (props interface from Figma
definitions).

## Troubleshooting

- **Auth error** — `fig2r auth "<token>"` or `FIGMA_TOKEN=...`. Scope:
  `file_content:read`.
- **Unsupported platform** — `npm install -g fig2r` to re-resolve optional
  platform deps.
- **Update notice** — `npm install -g fig2r@latest`. Silence with
  `NO_UPDATE_NOTIFIER=1` or `CI=1`.
- **Ambiguous output** — pipe through `fig2r convert --strict` to surface
  constructs that were silently coerced.

## Senior-dev tips

Short hits — details in [references/advanced-workflows.md](references/advanced-workflows.md):

- **Fetch per component, not per page** — smaller, diffable, reusable.
  Ask designer to select the component and copy link (URL gets a
  `?node-id=` suffix).
- **Design update? Re-fetch to a `-v2` dir and diff** — preserve handlers,
  state, tests, props, i18n bindings; apply only the deltas.
- **Responsive** — fetch mobile + desktop frames separately, merge with
  `md:` / `lg:` Tailwind prefixes (mobile-first) or `@container` queries.
- **Pull Figma variables via MCP** (`get_variable_defs`) — maps hex →
  exact token name, no guessing.
- **State variants** — fetch each (hover/disabled/loading/empty/error) or
  wire props via CVA. Reference shows happy path only.
- **Static content → props** — no literal "John Doe" / placeholder URLs
  in production. Extract to props or i18n keys.
- **Dark mode** — use theme-responsive tokens (`bg-background`), not
  `dark:bg-[#...]` for every hex.
- **Anchor comment** — `// figma: <file-key>/<node-id>` at the top of the
  component so future agents can re-fetch.
- **Lock it in** — write a Storybook story; add a visual-regression
  snapshot (Playwright / Chromatic).
- **CI drift detection** — check in IR JSON fixtures; re-fetch + diff in
  CI; fail with `fig2r convert --strict` on unsupported constructs.
- **Productionize images** — `<Image>` (next/image) over `<img>`; SVGO
  the custom SVGs; `priority` only on LCP.
- **A11y beyond tags** — contrast, focus rings, keyboard order, reduced
  motion, decorative `aria-hidden`.
- **Don't use fig2r for** — charts, complex animations, canvas/WebGL,
  rich-text editors, video/audio players, non-React targets.

## References

- [references/project-adaptation.md](references/project-adaptation.md) —
  stack detection commands; component-library mapping (shadcn / Radix /
  MUI / Chakra / Mantine); icon mapping; token mapping (Tailwind v3/v4,
  CSS vars); fonts; variants; a11y; asset dedupe + failed-download
  recovery; dark mode
- [references/advanced-workflows.md](references/advanced-workflows.md) —
  URL hygiene; incremental re-fetch / design updates; responsive
  multi-breakpoint; state variants; Figma variables via MCP; storybook +
  visual regression; CI drift detection; perf; i18n; when not to use fig2r
- [references/ir-schema.md](references/ir-schema.md) — IR JSON schema for
  the `convert` fallback path
