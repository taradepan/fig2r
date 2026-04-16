---
name: fig2r
description: Convert Figma nodes into React + Tailwind with asset and font handoff notes.
---

# fig2r

Use this when the user wants to convert a Figma URL (or node) into React + Tailwind output with usable assets.

## When to use

- User shares a Figma URL and asks for production-ready JSX/TSX
- User wants fig2r fetch/convert workflow help
- User needs handoff notes for assets/fonts/icons after generation

## Instructions

1. Run `fig2r fetch <figma-url> --save <components-dir>` to generate components and assets.
2. If the project has a Next.js `public/` folder, pass `--public-dir ./public` so assets land under `public/assets/`.
3. Ensure generated fonts are set up in `app/layout.tsx` (Google fonts + custom `@font-face` as needed).
4. Replace icon placeholders/vectors with the project’s icon library where appropriate.
5. Report any failed asset downloads and keep generated node comments for easier LLM edits.
