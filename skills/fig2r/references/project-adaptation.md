# Project Adaptation — Mapping fig2r Output to the Target Codebase

fig2r output is a **design spec**. Never ship it as-is. This guide covers
detecting the project's stack, then mapping fig2r's raw output to the
project's conventions.

## 1. Detect the stack

Run these before writing any code. Skipping this step = generic output that
won't match the project.

### Component library

```bash
# Check dependencies
cat package.json | grep -E '"(@radix-ui|@mui|@chakra-ui|@mantine|@headlessui|antd|@nextui|@arco|daisyui|@ariakit)"'

# Shadcn uses Radix + lives in project (not node_modules)
ls src/components/ui 2>/dev/null || ls components/ui 2>/dev/null
# If directory exists with button.tsx, card.tsx, etc. → shadcn

# shadcn's components.json at project root confirms it
cat components.json 2>/dev/null
```

### Styling

```bash
# Tailwind
ls tailwind.config.* 2>/dev/null
# Tailwind v4 uses @theme in CSS — check for it
grep -rln '@theme' --include='*.css' . 2>/dev/null | head -5

# CSS Modules / vanilla-extract / styled-components / emotion
cat package.json | grep -E '"(styled-components|@emotion|@vanilla-extract|@stitches|panda-css)"'
```

### Icons

```bash
cat package.json | grep -E '"(lucide-react|@heroicons|react-icons|@phosphor|@tabler|@radix-ui/react-icons|react-feather)"'
```

### Utilities

```bash
# cn() utility path — varies: @/lib/utils, ~/utils/cn, @/utils/cn
grep -rln 'export function cn\|export const cn' --include='*.ts' --include='*.tsx' . 2>/dev/null

# CVA (class-variance-authority) for variants
cat package.json | grep 'class-variance-authority'

# Path alias
cat tsconfig.json | grep -A2 '"paths"' 2>/dev/null
```

### Fonts

```bash
# Next.js: next/font
grep -rln 'from .next/font' --include='*.ts' --include='*.tsx' . 2>/dev/null | head -5

# Look for CSS variable-based font setup
grep -rln '\-\-font-' --include='*.css' . 2>/dev/null | head -5
```

### Design tokens

```bash
# Tailwind v3 extend
grep -A 20 'theme: *{' tailwind.config.* 2>/dev/null

# Tailwind v4 @theme
grep -A 40 '@theme' src/app/globals.css app/globals.css src/styles/globals.css 2>/dev/null

# CSS variables
grep -E '^\s*--(color|spacing|radius|font|shadow)' --include='*.css' -rh . 2>/dev/null | sort -u | head -50
```

Record findings in a short survey note before proceeding. Without this,
adaptation is guesswork.

## 2. Component-library mapping

fig2r outputs raw `<div>` elements with Tailwind classes. Replace them with
project components wherever a match exists.

### Shadcn / ui

| fig2r output | Shadcn component |
|---|---|
| `<div>` + button classes (bg, padding, rounded, hover) | `<Button variant="...">` |
| `<div>` + card classes (bg, border, rounded-lg, padding) | `<Card>` with `<CardHeader>` / `<CardContent>` / `<CardFooter>` |
| `<div>` with avatar image + rounded-full | `<Avatar>` / `<AvatarImage>` / `<AvatarFallback>` |
| small pill with colored bg + rounded-full | `<Badge variant="...">` |
| `<hr>` or thin horizontal line | `<Separator>` |
| input fields | `<Input>`, `<Textarea>`, `<Select>` |
| `<img>` placeholder circles | `<Avatar>` |
| checkmark/x icon in styled container | `<Checkbox>` / `<Switch>` (if interactive) |
| tooltip-like overlay | `<Tooltip>` / `<HoverCard>` / `<Popover>` |

Detect variants by comparing fig2r's colors/borders against the shadcn
component's variant definitions (usually in `src/components/ui/button.tsx`
etc.). Match by token, not by hex.

### Radix primitives (without shadcn)

Similar mapping — the component surface is identical, the project just
styles them directly. Check `components/` for existing Radix wrappers
before importing from `@radix-ui/*` fresh.

### MUI

| fig2r output | MUI component |
|---|---|
| button-styled div | `<Button variant="contained\|outlined\|text">` |
| card-styled div | `<Card>` + `<CardContent>` |
| chip/badge | `<Chip>` |
| input field | `<TextField>` |

Use `sx` prop for overrides — don't fight MUI's theme by using arbitrary
Tailwind classes unless the project mixes them.

### Chakra / Mantine

Map similarly. Chakra uses `<Button>`, `<Box>`, `<Flex>`, `<Stack>`.
Mantine uses `<Button>`, `<Paper>`, `<Group>`, `<Stack>`.

**Don't recreate a primitive the project already has.** If the project has
its own `<Button>`, `<Card>`, etc. in `src/components/`, use those even if
they wrap a library.

## 3. Icon-library mapping

fig2r exports every icon as SVG. Most match standard library icons.

```bash
# List all icons fig2r exported
ls /tmp/fig2r-output/assets/*.svg /tmp/fig2r-output/icons/*.tsx 2>/dev/null
```

| Figma name contains | Likely library icon |
|---|---|
| check, checkmark, tick | `Check`, `CheckCircle` |
| close, x, xmark | `X`, `XCircle` |
| chevron-down / caret | `ChevronDown` |
| arrow-right | `ArrowRight`, `ArrowLeft`, etc. |
| search, magnifier | `Search` |
| user, avatar, person | `User` |
| settings, gear, cog | `Settings` |
| home, house | `Home` |
| menu, hamburger | `Menu` |
| plus, add | `Plus` |
| minus, remove | `Minus` |
| star, favorite | `Star` |
| heart, like | `Heart` |
| trash, delete, bin | `Trash`, `Trash2` |
| edit, pencil | `Pencil`, `Edit`, `Edit2` |
| copy | `Copy` |
| download, upload | `Download`, `Upload` |
| eye, view, visibility | `Eye`, `EyeOff` |
| bell, notification | `Bell` |
| external-link, open | `ExternalLink` |

Match size + color:
```tsx
// fig2r emits:  <img src="/assets/check.svg" className="w-5 h-5" />
// Replace with: <Check className="w-5 h-5 text-foreground" />
// or:           <Check size={20} />
```

Keep SVG files ONLY for custom illustrations, logos, or icons with no
library equivalent.

## 4. Token mapping

fig2r emits raw hex (`bg-[#3A422C]`), raw px (`p-[13px]`), and arbitrary
values. Replace with project tokens wherever they match.

### Mapping heuristic

1. Fetch all hex values from the reference:
   ```bash
   grep -oE '#[0-9A-Fa-f]{3,8}' /tmp/fig2r-output/**/*.tsx | sort -u
   ```
2. For each hex, find a matching token:
   - Tailwind v3 config: check `theme.extend.colors`
   - Tailwind v4: check `@theme` in CSS — `--color-brand: #3A422C` → `bg-brand`
   - CSS variables: `var(--accent)` → `bg-[var(--accent)]` or token alias
3. Exact-match replace. Near-match (within 2-3 units) = still replace;
   designers often round. Very different = keep the hex and flag as a
   new color the design uses.

### Spacing / radii / font-size

Same pattern:
- `rounded-[8px]` → `rounded-lg` (if project defines lg as 0.5rem/8px)
- `text-[14px]` → `text-sm` (if project keeps default sm as 14px)
- `p-[16px]` → `p-4` (default 1rem)

If the project overrides default Tailwind scales, check the config first.

### Font families

- **next/font with `variable:` option**: use `font-[var(--font-xxx)]`
  (literal family names break SSR/CLS optimizations)
- **Raw CSS @font-face / Google Fonts link**: `font-[family,\_fallback]`
  syntax works
- **Tailwind `theme.fontFamily.*`**: `font-sans`, `font-display`, etc.

Never emit a literal `"Inter", sans-serif` in className — it bypasses
the font pipeline the project set up.

### Tailwind v4 specifics

v4 uses `@theme` in CSS, not `tailwind.config.ts`. Token syntax:
- `--color-*` → `bg-*`, `text-*`, `border-*`
- `--spacing-*` → `p-*`, `m-*`, `gap-*`
- `--radius-*` → `rounded-*`
- `--font-*` → `font-*`

Arbitrary values still work (`bg-[#...]`) but prefer tokens.

## 5. Variants & props

If the Figma component has variants (size=sm/md/lg, intent=primary/secondary),
fig2r emits a props interface. Wire this to the project's variant system:

- **CVA**: define a `cva()` block, map variants to Tailwind classes
- **Shadcn**: edit the existing variant file (usually colocated)
- **MUI/Chakra/Mantine**: map to their built-in size/color props

## 6. Interactivity

fig2r output is static — no handlers, no state, no forms. Add:
- `onClick` / `onChange` / `onSubmit`
- `useState`, `useReducer`, or project's state solution (Zustand, Redux, etc.)
- Form libs (react-hook-form, formik)
- Routing (next/link, react-router)
- Data fetching (fetch, SWR, TanStack Query, Apollo)

Match the project's patterns — don't introduce a new state library.

## 7. A11y check

fig2r emits semantic tags (`button`, `nav`, `header`) from Figma layer names,
but designers don't always name correctly. Verify:
- Buttons are `<button>`, not `<div onClick>`
- Links are `<a>` / `<Link>`, not `<div onClick>`
- Form inputs have labels
- Icons without text have `aria-label`
- Heading levels follow document outline

## 8. Asset handling — dedupe + failed downloads

### Dedupe before copying

fig2r writes every raster/SVG into `/tmp/fig2r-output/assets/`. The project
likely already has some of them (logos, avatars, illustrations reused
across pages). Copying blindly creates duplicates with different names.

**Hash-compare** against existing project assets:

```bash
# Build a map of existing hashes
find public -type f \( -name '*.png' -o -name '*.jpg' -o -name '*.jpeg' \
  -o -name '*.webp' -o -name '*.svg' -o -name '*.avif' \) \
  -exec shasum -a 256 {} + > /tmp/project-asset-hashes.txt 2>/dev/null

# For each new asset, check for a dupe
for f in /tmp/fig2r-output/assets/*; do
  [ -f "$f" ] || continue
  h=$(shasum -a 256 "$f" | cut -d' ' -f1)
  match=$(grep "^$h " /tmp/project-asset-hashes.txt | head -1 | awk '{print $2}')
  if [ -n "$match" ]; then
    echo "DUPE: $(basename "$f")  →  ${match#public/}"
  else
    echo "NEW:  $(basename "$f")"
  fi
done
```

For each `DUPE`, rewrite the `<img src="/assets/foo.png">` in the adapted
component to point at the existing path (e.g., `/brand/logo.png`). Do not
copy the fig2r file.

**Filename-match fallback** (for cases where bytes differ but the image is
semantically the same — e.g., re-exported at a different scale):

```bash
for f in /tmp/fig2r-output/assets/*; do
  name=$(basename "$f")
  # Strip fig2r's hash suffix if any (e.g., logo-a1b2c3.png → logo.png)
  stem="${name%.*}"; ext="${name##*.}"
  base="${stem%-*}"
  find public -iname "${base}.*" -o -iname "${base}-*.${ext}" 2>/dev/null
done
```

Review matches manually before trusting — same name ≠ same image.

### Failed downloads — recovery

`[SUMMARY] ... failed_downloads=N` with `N > 0` means N assets are
referenced in the generated `.tsx` but the files were not written.
Shipping as-is = runtime 404.

**Recovery order:**

1. **Retry fetch** — Figma image CDN (S3) occasionally 5xx's. Re-run:
   ```bash
   fig2r fetch "<url>" --save /tmp/fig2r-output
   ```
   Transient failures clear on second attempt ~80% of the time.

2. **Figma MCP** (if installed in the agent environment):
   ```
   # Check availability
   mcp__plugin_figma_figma__get_design_context
   mcp__plugin_figma_figma__get_screenshot
   ```
   Pass the nodeId of the missing asset; save the returned image under
   the project's public dir. URL parsing: `figma.com/design/:fileKey/...`
   with `?node-id=X-Y` → convert `-` to `:` (e.g., `4563-6632` →
   `4563:6632`).

3. **Direct download from `asset.url`** — fig2r stores the Figma-issued
   S3 URL in the IR even when the download fails. If you can run a web
   fetch tool, pull the image directly:
   ```bash
   # The URL is visible in the generated .tsx or in IR JSON output
   curl -fL "<asset.url>" -o public/assets/<name>
   ```
   Note: Figma image URLs are pre-signed and expire — retry quickly.

4. **Ask the user** — if all automated paths fail, request the asset and
   the target path.

**Never ship with `failed_downloads > 0` unresolved.** The `<img>` tags in
the component still reference the missing files, so the app renders broken
images in production.

### Tracking missing assets

Grep the generated `.tsx` for every asset reference, cross-check against
disk:

```bash
grep -oE '/assets/[^"]+' /tmp/fig2r-output/**/*.tsx | sort -u > /tmp/refs.txt
while read ref; do
  [ -f "public${ref}" ] || echo "MISSING: $ref"
done < /tmp/refs.txt
```

Every `MISSING` must be resolved before declaring the task done.

## 9. Dark mode / themes

fig2r fetches a single variant. If the project has dark mode, check whether
the Figma file has a dark variant (separate node). Fetch both, then map
colors to theme tokens (`bg-background` / `text-foreground`) that respond to
the project's theme system.
