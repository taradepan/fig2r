# Advanced Workflows

Senior-dev patterns for using fig2r at full potential. Load when the basic
workflow in SKILL.md isn't enough — multi-variant designs, design updates,
visual regression, CI gating, etc.

## 1. URL hygiene — fetch the right node

A Figma URL without `?node-id=` fetches the entire page = hundreds of
components = unusable output. Always:

- Ask the designer to **select the component** and copy link. The link
  will include `?node-id=X-Y`.
- If given a page-level URL, open it, select the component yourself, then
  copy link.
- URL node IDs use `-` as separator; the Figma API (and some tools) use
  `:`. fig2r handles both. For Figma MCP calls, convert (`4563-6632` →
  `4563:6632`).

Smaller, scoped fetches produce tight components that diff cleanly on
updates.

## 2. Component-level, not page-level

Instead of one big `fig2r fetch <page-url>`, fetch each component node
separately:

```bash
fig2r fetch "<header-url>" --save /tmp/fig2r-header
fig2r fetch "<hero-url>"   --save /tmp/fig2r-hero
fig2r fetch "<cta-url>"    --save /tmp/fig2r-cta
```

Benefits:
- Each reference is small and reviewable
- Implementation files map 1:1 to source components
- Re-fetching one updated component doesn't touch others
- Asset dedupe works across sibling fetches

## 3. Incremental re-fetch — design updates

Designer changed the hero. Don't rebuild from scratch.

```bash
# Fetch into a new dir
fig2r fetch "<hero-url>" --save /tmp/fig2r-hero-v2

# Diff against the previous fetch
diff -r /tmp/fig2r-hero /tmp/fig2r-hero-v2 | head -200

# Apply ONLY the deltas to the implemented component — preserve:
#   - interactivity (handlers, state)
#   - tests
#   - props interface (unless design changed it)
#   - i18n bindings
#   - imports from project components
```

Keep `/tmp/fig2r-<component>-v<N>/` around during the change so you can
re-diff after each adaptation pass.

## 4. Responsive designs (mobile + desktop)

Figma typically has separate frames per breakpoint. Fetch each, merge
into one responsive component:

```bash
fig2r fetch "<mobile-hero-url>"  --save /tmp/fig2r-hero-mobile
fig2r fetch "<desktop-hero-url>" --save /tmp/fig2r-hero-desktop
```

Merge strategy (pick one based on project conventions):

**A. Mobile-first with Tailwind breakpoint prefixes** (most common):
- Base classes = mobile
- `md:` / `lg:` prefixes = overrides from desktop reference
- Use `--responsive` flag on fetch to get `w-full max-w-[Npx]` root

**B. Container queries** (if project uses `@tailwindcss/container-queries`
or native `@container`):
- `@container` on wrapper
- `@md:` / `@lg:` on children

**C. Separate components** (rarely justified — when mobile and desktop
have fundamentally different layouts):
- `<HeroMobile>` + `<HeroDesktop>` with SSR-safe hydration
- Hide via `hidden md:block` / `md:hidden`

Compare the two references side by side:
```bash
diff <(grep -oE 'className="[^"]+"' /tmp/fig2r-hero-mobile/**/*.tsx) \
     <(grep -oE 'className="[^"]+"' /tmp/fig2r-hero-desktop/**/*.tsx)
```

## 5. State variants (hover, disabled, loading, empty, error)

fig2r fetches one variant. For stateful components:

- Fetch **each variant** as a separate node if the designer made them
  separate frames (common in well-organized design systems).
- Compare references to extract:
  - Hover/focus → Tailwind `hover:` / `focus-visible:` classes
  - Disabled → `disabled:` + `aria-disabled`
  - Loading → skeleton, spinner component, or `aria-busy`
  - Empty → illustration + CTA
  - Error → color shift + error message slot
- If variants are via Figma component properties (not separate frames),
  fig2r emits a props interface. Wire each prop to the correct variant
  classes, ideally via CVA.

## 6. Figma variables (design tokens) via MCP

If Figma MCP is available, pull variables BEFORE or ALONGSIDE the fetch:

```
mcp__plugin_figma_figma__get_variable_defs
  fileKey: <from-url>
  nodeId:  <component-node>
```

Returns the Figma variables used by this component (color, number, string
types). These map 1:1 to the project's design tokens:

- `color/primary/default` → `--color-primary` / `bg-primary`
- `spacing/md` → `--spacing-md` / `p-md`
- `radius/lg` → `--radius-lg` / `rounded-lg`

When fig2r emits `bg-[#3A422C]` and Figma variable `color/brand/default =
#3A422C`, the correct token name is revealed. Skip guessing by hex
proximity.

Same tool also exposes variable modes (light/dark, compact/roomy) — use
these to drive project theme switching.

## 7. Static content → props

fig2r's reference embeds literal strings ("John Doe", "Welcome to…") and
asset paths. Productionize:

- Extract every literal string into a prop or i18n key
- Extract images as props if the same component renders different images
  (e.g., product card)
- Replace placeholder URLs with real data-fetching calls or prop passthrough
- Preserve text for a11y (don't lose `aria-label` content)

```tsx
// Before (from reference)
<h1>Welcome back, John</h1>
<img src="/assets/avatar-placeholder.png" />

// After (productionized)
<h1>{t('greeting', { name: user.name })}</h1>
<Avatar src={user.avatarUrl} fallback={user.initials} />
```

## 8. Dark mode / theme variants

Two paths:

**Path A: single component + theme tokens** (preferred):
- Implement with theme-responsive tokens: `bg-background`, `text-foreground`,
  `border-border`
- Verify both modes render correctly by toggling the theme
- If Figma has separate light/dark frames, fetch both and use as a check
  that your tokens produce each result

**Path B: variant prop** (rare — when dark mode diverges structurally):
- `<Component theme="light" />` / `<Component theme="dark" />`
- Use only when theme tokens can't express the difference

Never inline `dark:` classes for every hex — that's Path A done wrong.

## 9. Post-implementation: Storybook story

Lock the design in a story so future regressions are visible:

```tsx
// Button.stories.tsx
import { Button } from './Button';
import type { Meta, StoryObj } from '@storybook/react';

const meta = { component: Button, parameters: { figma: { url: '<figma-url>' }}};
export default meta;

export const Primary: StoryObj<typeof Button> = {
  args: { variant: 'primary', children: 'Continue' },
};
export const Disabled: StoryObj<typeof Button> = {
  args: { variant: 'primary', disabled: true, children: 'Continue' },
};
```

Storybook's Figma addon then shows the source design next to the rendered
component for ongoing visual QA.

## 10. Visual regression testing

Confirm pixel-level match automatically:

```bash
# Playwright + storybook-screenshot or Chromatic
# Snapshot the story, diff against a previous baseline
npx playwright test --update-snapshots   # first time
npx playwright test                      # on PR
```

For one-off checks, compare Figma's rendered screenshot against the local
component:

```
mcp__plugin_figma_figma__get_screenshot   # Figma export
# vs
mcp__plugin_playwright_playwright__browser_take_screenshot   # local
```

Pixel-diff tools: `pixelmatch`, `odiff`, or Chromatic's visual review.

## 11. Anchor comment — traceability

Add a tiny comment tying the component back to Figma:

```tsx
// figma: KEY/node-id (e.g., https://www.figma.com/design/abc.../def?node-id=45-123)
export function Hero() { ... }
```

Why: next agent (or future you) can re-fetch to compare against current
state without hunting for the URL. Keep it as a URL comment, not a `@link`
docblock — it's metadata, not documentation.

## 12. CI: detect design drift

Check in an IR JSON snapshot per component. In CI, re-fetch and diff:

```bash
# In repo: tests/fixtures/hero.ir.json   (last known good)
# In CI:
fig2r fetch "<hero-url>" > /tmp/hero.ir.json
fig2r convert /tmp/hero.ir.json -o /tmp/hero-out --strict

# Diff against committed snapshot
diff <(jq -S . tests/fixtures/hero.ir.json) \
     <(jq -S . /tmp/hero.ir.json)
```

- `--strict` surfaces newly-unsupported constructs (fail build)
- Diff output tells design team exactly what changed, before code review
- Bonus: auto-open PR when drift detected, with the new IR and a
  regenerated reference

## 13. Performance productionization

fig2r output is naive. Before shipping:

- **Next.js**: replace `<img>` with `<Image>` from `next/image`. Set
  `width`/`height` (fig2r's bounding boxes give you both) and `sizes`.
- **SVG optimization**: run `svgo` on custom SVGs before committing. Drop
  `<metadata>`, `<defs>` garbage Figma emits.
- **Lazy-load below-the-fold images**: `loading="lazy"` or `<Image priority />`
  for only the LCP image.
- **Font loading**: `display: 'swap'` for variable fonts, or `'optional'`
  for non-critical.
- **Code splitting**: if the component is heavy (chart, rich text), wrap
  in `React.lazy` + `<Suspense>`.

## 14. A11y beyond semantic HTML

Semantic tags are necessary but not sufficient. Also check:

- **Contrast ratio** — 4.5:1 text, 3:1 UI. Designer can ship low-contrast
  combos; run `axe` or similar after adapting.
- **Focus states** — every interactive element visible on keyboard focus.
  fig2r doesn't emit `focus-visible:ring-*`; add it.
- **Keyboard nav** — tab order matches visual order; no keyboard traps.
  Forms submit on Enter.
- **ARIA** — only when native semantics are insufficient. Don't spray
  `role="button"` on divs; use `<button>`.
- **Reduced motion** — `prefers-reduced-motion: reduce` for any animation.
- **Screen reader** — decorative icons get `aria-hidden="true"`; meaningful
  icons get `aria-label`.

## 15. When NOT to use fig2r

fig2r is for static UI. Skip it for:

- **Charts / data viz** — use d3, Recharts, Chart.js. Figma mock is
  reference for visual style only.
- **Complex animations / motion** — Framer Motion, GSAP, Rive. Figma's
  prototype arrows don't encode timing/easing.
- **Canvas / WebGL / 3D** — obviously.
- **Emoji-as-icons** — if the "icon" is a Unicode character in Figma,
  it'll export as a text glyph, not an SVG. Replace manually.
- **Rich-text editors** — fig2r emits static text. Use TipTap/Slate/Lexical.
- **Video/audio players** — use the platform's native controls or a
  library (video.js). Fig2r output will be hollow shells.
- **Non-React targets** — fig2r only emits React+Tailwind. For Vue/Svelte/
  Solid, translate manually or use a different tool.

## 16. i18n integration

If the project has i18n (next-intl, react-i18next, lingui, etc.):

- Every literal string from the reference → `t('<key>')`
- Keys named by function, not by text: `cta.signup`, not `sign_up_button`
- Pluralization via the library's pluralization API, not ternaries
- Keep the English copy in the source file of truth; don't rely on the
  Figma text as canonical (design copy lags production)

## 17. Design-system contribution

If the adapted component introduces a pattern not yet in the project's
design system:

1. Check: is this a one-off (keep local) or a reusable primitive (upstream)?
2. If reusable, add to `components/ui/` or equivalent, with props /
   variants / stories / tests
3. Update the design system's docs if one exists
4. Notify the design team that this primitive now exists, so they can use
   it in future Figma designs instead of re-drawing

The goal: Figma and code converge on the same set of primitives.
