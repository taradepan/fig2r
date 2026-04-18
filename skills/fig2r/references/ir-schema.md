# fig2r IR JSON Schema

Complete schema for constructing valid IR JSON input for fig2r.

## Table of Contents

- [Top-Level Structure](#top-level)
- [Theme](#theme)
- [Node](#node)
- [Layout](#layout)
- [Style](#style)
- [Text](#text)
- [Vector](#vector)
- [Boolean Operations](#boolean-operations)
- [Mask](#mask)
- [Component (Variants)](#component)
- [Asset](#asset)
- [Mapping Figma MCP Data to IR](#mapping-figma-mcp)

## Top-Level

```json
{
  "version": "1.0",
  "name": "DesignExportName",
  "theme": { },
  "components": [ ],
  "assets": [ ]
}
```

- `version` (string, required): Always `"1.0"`
- `name` (string, required): Name for this export
- `theme` (object, optional): Design tokens
- `components` (array of Node): Top-level frames/components to generate
- `assets` (array of Asset): Images and SVGs

## Theme

All fields optional. Maps to `tailwind.extend.js` + `tokens.ts`.

```json
{
  "colors": { "primary": "#3B82F6", "primary-hover": "#2563EB" },
  "spacing": { "sm": "4px", "md": "8px", "lg": "16px" },
  "borderRadius": { "sm": "4px", "md": "8px", "full": "9999px" },
  "fontSize": { "xs": "12px", "sm": "14px", "base": "16px" },
  "fontFamily": { "sans": "Inter", "mono": "JetBrains Mono" },
  "shadows": { "sm": "0 1px 2px rgba(0,0,0,0.05)" },
  "opacity": { "disabled": 0.5 }
}
```

When theme colors are present, fig2r uses them in output (e.g., `bg-primary` instead of `bg-[#3B82F6]`).

## Node

Every element in the design tree.

```json
{
  "id": "unique-id",
  "name": "ComponentName",
  "type": "frame",
  "layout": { },
  "style": { },
  "text": { },
  "vector": { },
  "booleanOp": { },
  "mask": { },
  "component": { },
  "children": [ ]
}
```

- `id` (string, required): Unique identifier
- `name` (string, required): Used as React component/element name
- `type` (string, required): `frame` | `text` | `image` | `vector` | `group` | `instance` | `boolean_op`
- `children` (array of Node, required): Child nodes (empty array if leaf)
- All other fields optional

## Layout

```json
{
  "mode": "horizontal",
  "width": { "type": "fixed", "value": 320 },
  "height": { "type": "hug" },
  "padding": { "top": 16, "right": 16, "bottom": 16, "left": 16 },
  "gap": 8,
  "mainAxisAlign": "center",
  "crossAxisAlign": "stretch",
  "constraints": { "horizontal": "stretch", "vertical": "top" },
  "position": { "x": 0, "y": 0 },
  "overflow": "hidden"
}
```

**mode**: `horizontal` | `vertical` | `none`

**width/height type**: `fixed` | `fill` | `hug`
- `fixed`: uses `value` field (pixels)
- `fill`: expands to parent (`w-full`)
- `hug`: shrinks to content (`w-fit`)

**mainAxisAlign**: `start` | `center` | `end` | `space-between` | `stretch`

**crossAxisAlign**: `start` | `center` | `end` | `space-between` | `stretch`

**constraints horizontal**: `left` | `right` | `center` | `stretch`

**constraints vertical**: `top` | `bottom` | `center` | `stretch`

**overflow**: `visible` | `hidden` | `scroll`

## Style

```json
{
  "fills": [ ],
  "stroke": { "color": "#E5E7EB", "width": 1, "position": "inside" },
  "borderRadius": { "topLeft": 8, "topRight": 8, "bottomRight": 8, "bottomLeft": 8 },
  "effects": [ ],
  "opacity": 0.9,
  "blendMode": "normal"
}
```

### Fills (array)

Three types, discriminated by `type` field:

**Solid:**
```json
{ "type": "solid", "color": "#3B82F6", "opacity": 1.0 }
```

**Gradient:**
```json
{
  "type": "gradient",
  "gradientType": "linear",
  "stops": [
    { "position": 0.0, "color": "#3B82F6" },
    { "position": 1.0, "color": "#8B5CF6" }
  ]
}
```
`gradientType`: `linear` | `radial` | `angular` (angular emits a warning, approximated as linear)

**Image:**
```json
{ "type": "image", "assetRef": "asset-id-here" }
```

### Stroke

```json
{ "color": "#E5E7EB", "width": 1, "position": "inside" }
```
`position`: `inside` | `outside` | `center`

### Effects (array)

**Drop shadow:**
```json
{ "type": "drop-shadow", "offset": { "x": 0, "y": 4 }, "radius": 6, "spread": 0, "color": "rgba(0,0,0,0.1)" }
```

**Inner shadow:**
```json
{ "type": "inner-shadow", "offset": { "x": 0, "y": 2 }, "radius": 4, "spread": 0, "color": "rgba(0,0,0,0.05)" }
```

**Blur:**
```json
{ "type": "blur", "blurType": "layer", "radius": 8 }
```
`blurType`: `layer` | `background`

### blendMode

`normal` | `multiply` | `screen` | `overlay` (non-normal emits warning)

## Text

Only on nodes with `"type": "text"`.

```json
{
  "content": "Button Label",
  "fontSize": 16,
  "fontFamily": "Inter",
  "fontWeight": 600,
  "lineHeight": 1.5,
  "letterSpacing": 0.05,
  "textAlign": "center",
  "textDecoration": "underline",
  "textTransform": "uppercase",
  "truncation": "ellipsis"
}
```

- `content` (string, required): The text content
- `fontSize` (number): In pixels
- `fontWeight` (integer): 100-900
- `textAlign`: `left` | `center` | `right` | `justify`
- `textDecoration`: `none` | `underline` | `strikethrough`
- `textTransform`: `none` | `uppercase` | `lowercase` | `capitalize`
- `truncation`: `none` | `ellipsis`

## Vector

Only on nodes with `"type": "vector"`.

```json
{
  "svgPath": "M10 20 L30 40 Z",
  "fillRule": "nonzero"
}
```

- `svgPath` (string, required): SVG path data
- `fillRule`: `nonzero` | `evenodd`

## Boolean Operations

Only on nodes with `"type": "boolean_op"`.

```json
{
  "booleanOp": {
    "operation": "union",
    "children": [ ]
  }
}
```

`operation`: `union` | `subtract` | `intersect` | `exclude`

`children`: Array of Node (the shapes being combined)

## Mask

```json
{
  "mask": {
    "isMask": true,
    "maskType": "alpha"
  }
}
```

`maskType`: `alpha` | `vector`

## Component

Marks a node as a Figma component with variants.

```json
{
  "component": {
    "isComponent": true,
    "variants": {
      "size": ["sm", "md", "lg"],
      "variant": ["primary", "secondary"]
    },
    "variantValues": {
      "size": "md",
      "variant": "primary"
    }
  }
}
```

- `isComponent` (bool): Whether this is a component definition
- `variants`: Map of prop name → possible values
- `variantValues`: Map of prop name → default value for this instance

## Asset

```json
{
  "id": "asset-123",
  "name": "hero-image",
  "type": "image",
  "format": "png",
  "data": "base64-encoded-data"
}
```

- `id` (string, required): Referenced by `Fill.Image.assetRef`
- `name` (string, required): Used for filename
- `type`: `image` | `svg`
- `format`: `png` | `jpg` | `webp` | `svg`
- `data`: Base64-encoded binary for images, raw SVG markup for SVGs

## Mapping Figma MCP Data to IR

When transforming Figma MCP responses to fig2r IR:

1. **Frames** → Node with `type: "frame"`, extract auto-layout as `layout`
2. **Text layers** → Node with `type: "text"`, extract text properties
3. **Rectangles/shapes** → Node with `type: "frame"` (they're just styled containers)
4. **Components** → Node with `component.isComponent: true`, map Figma variants
5. **Instances** → Node with `type: "instance"`, include `component.variantValues`
6. **Vectors** → Node with `type: "vector"`, extract SVG path data
7. **Groups** → Node with `type: "group"`, children as-is
8. **Boolean groups** → Node with `type: "boolean_op"`
9. **Images** → Add to `assets` array with base64 data, reference via `Fill.Image`
10. **Design tokens/variables** → Extract into `theme` object

### Figma Properties to IR Fields

| Figma MCP Field | IR Field |
|---|---|
| `absoluteBoundingBox.width` | `layout.width.value` (with `type: "fixed"`) |
| `absoluteBoundingBox.height` | `layout.height.value` (with `type: "fixed"`) |
| `layoutMode: "HORIZONTAL"` | `layout.mode: "horizontal"` |
| `layoutMode: "VERTICAL"` | `layout.mode: "vertical"` |
| `itemSpacing` | `layout.gap` |
| `paddingTop/Right/Bottom/Left` | `layout.padding` |
| `primaryAxisAlignItems` | `layout.mainAxisAlign` |
| `counterAxisAlignItems` | `layout.crossAxisAlign` |
| `layoutSizingHorizontal: "FILL"` | `layout.width.type: "fill"` |
| `layoutSizingHorizontal: "HUG"` | `layout.width.type: "hug"` |
| `fills[].type: "SOLID"` | `style.fills[].type: "solid"` |
| `fills[].color` | `style.fills[].color` (convert to hex) |
| `cornerRadius` | `style.borderRadius` (all corners) |
| `rectangleCornerRadii` | `style.borderRadius` (per-corner) |
| `effects[].type: "DROP_SHADOW"` | `style.effects[].type: "drop-shadow"` |
| `characters` | `text.content` |
| `style.fontSize` | `text.fontSize` |
| `style.fontWeight` | `text.fontWeight` |
