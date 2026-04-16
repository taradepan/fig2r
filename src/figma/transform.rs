use crate::figma::types::{BoundingBox, FigmaNode, FigmaPaint};
use crate::ir::schema::{
    Alignment, Asset, AssetType, BlendMode, BorderRadius, ComponentInfo, DesignIR, Dimension,
    DimensionType, Effect, Fill, GradientStop, GradientType, Layout, LayoutMode, ListType, Mask,
    MaskType, Node, NodeType, Overflow, Padding, Position, ScaleMode, Stroke, StrokePosition,
    Style, TextAlign, TextDecoration, TextProps, TextSpan, TextTransform, Truncation,
    VerticalAlign,
};
use std::collections::HashMap;

/// Round a pixel value to the nearest 0.5px.
fn round_half_px(v: f64) -> f64 {
    (v * 2.0).round() / 2.0
}

/// Round a value to `n` decimal places (used for text metrics like line-height ratios).
fn round_decimal(v: f64, n: u32) -> f64 {
    let factor = 10_f64.powi(n as i32);
    (v * factor).round() / factor
}

fn parse_blend_mode(mode: Option<&str>) -> Option<BlendMode> {
    match mode? {
        "NORMAL" | "PASS_THROUGH" => None,
        "MULTIPLY" => Some(BlendMode::Multiply),
        "SCREEN" => Some(BlendMode::Screen),
        "OVERLAY" => Some(BlendMode::Overlay),
        "DARKEN" => Some(BlendMode::Darken),
        "LIGHTEN" => Some(BlendMode::Lighten),
        "COLOR_DODGE" => Some(BlendMode::ColorDodge),
        "COLOR_BURN" => Some(BlendMode::ColorBurn),
        "HARD_LIGHT" => Some(BlendMode::HardLight),
        "SOFT_LIGHT" => Some(BlendMode::SoftLight),
        "DIFFERENCE" => Some(BlendMode::Difference),
        "EXCLUSION" => Some(BlendMode::Exclusion),
        "HUE" => Some(BlendMode::Hue),
        "SATURATION" => Some(BlendMode::Saturation),
        "COLOR" => Some(BlendMode::Color),
        "LUMINOSITY" => Some(BlendMode::Luminosity),
        _ => None,
    }
}

/// Scale components of a 2D affine matrix — sign encodes flip, magnitude encodes scale.
/// Rotation and translation are recovered through other paths (node rotation / absolute
/// bounding box) so we don't keep them here.
#[derive(Debug, Default)]
struct DecomposedTransform {
    scale_x: f64,
    scale_y: f64,
}

fn decompose_matrix(m: &[[f64; 3]; 2]) -> DecomposedTransform {
    let a = m[0][0];
    let c = m[0][1];
    let b = m[1][0];
    let d = m[1][1];

    let sx = (a * a + b * b).sqrt();
    let det = a * d - b * c;
    let sy = (c * c + d * d).sqrt() * if det < 0.0 { -1.0 } else { 1.0 };

    DecomposedTransform {
        scale_x: sx,
        scale_y: sy,
    }
}

fn parse_scale_mode(mode: Option<&str>) -> Option<ScaleMode> {
    match mode? {
        "FILL" => Some(ScaleMode::Fill),
        "FIT" => Some(ScaleMode::Fit),
        "CROP" => Some(ScaleMode::Crop),
        "TILE" => Some(ScaleMode::Tile),
        _ => None,
    }
}

/// Build text spans from Figma's `characterStyleOverrides` + `styleOverrideTable`.
/// Groups consecutive characters with the same override key into styled runs.
/// Returns None when there are no per-character overrides (all zeros or missing).
fn build_text_spans(
    figma: &FigmaNode,
    base: &crate::figma::types::FigmaTypeStyle,
) -> Option<Vec<TextSpan>> {
    let chars = figma.characters.as_ref()?;
    let overrides = figma.character_style_overrides.as_ref()?;
    let table = figma.style_override_table.as_ref()?;
    if chars.is_empty() || overrides.iter().all(|&k| k == 0) {
        return None;
    }

    // Walk characters, group by override key
    let char_vec: Vec<char> = chars.chars().collect();
    let mut spans: Vec<TextSpan> = Vec::new();
    let mut current_key = overrides.first().copied().unwrap_or(0);
    let mut current_text = String::new();

    let make_span = |key: u32, text: String| -> TextSpan {
        let override_style = if key == 0 {
            None
        } else {
            table.get(&key.to_string())
        };
        // Merge: base is used for defaults; override (if present) wins.
        // Only include fields where the override DIFFERS from base, to keep output clean.
        let font_weight = override_style
            .and_then(|o| o.font_weight)
            .filter(|w| base.font_weight.is_none_or(|b| (b - w).abs() > 0.5))
            .map(|w| w as u32);
        let italic = override_style
            .and_then(|o| o.italic)
            .filter(|&i| base.italic != Some(i));
        let text_decoration = override_style.and_then(|o| {
            o.text_decoration.as_deref().and_then(|d| match d {
                "UNDERLINE" => Some(TextDecoration::Underline),
                "STRIKETHROUGH" => Some(TextDecoration::Strikethrough),
                _ => None,
            })
        });
        let font_family = override_style
            .and_then(|o| o.font_family.clone())
            .filter(|f| base.font_family.as_deref() != Some(f.as_str()));
        let font_size = override_style
            .and_then(|o| o.font_size)
            .filter(|s| base.font_size.is_none_or(|b| (b - s).abs() > 0.5));
        let color = override_style.and_then(|o| {
            o.fills
                .as_ref()?
                .iter()
                .find(|f| f.visible)
                .and_then(resolve_paint_color)
        });
        let hyperlink = override_style
            .and_then(|o| o.hyperlink.as_ref())
            .and_then(|h| h.get("url").and_then(|v| v.as_str()))
            .map(String::from);

        TextSpan {
            content: text,
            font_weight,
            italic,
            text_decoration,
            font_family,
            font_size,
            color,
            hyperlink,
        }
    };

    for (i, ch) in char_vec.iter().enumerate() {
        let key = overrides.get(i).copied().unwrap_or(0);
        if key != current_key {
            spans.push(make_span(current_key, std::mem::take(&mut current_text)));
            current_key = key;
        }
        current_text.push(*ch);
    }
    if !current_text.is_empty() {
        spans.push(make_span(current_key, current_text));
    }

    // If only one span with no overrides, skip (not useful)
    if spans.len() == 1
        && spans[0].font_weight.is_none()
        && spans[0].italic.is_none()
        && spans[0].text_decoration.is_none()
        && spans[0].font_family.is_none()
        && spans[0].font_size.is_none()
        && spans[0].color.is_none()
        && spans[0].hyperlink.is_none()
    {
        return None;
    }

    Some(spans)
}

/// Entry point: convert a top-level Figma node into a DesignIR.
pub fn figma_to_ir(name: &str, figma_node: &FigmaNode) -> DesignIR {
    let mut assets = Vec::new();
    let root = transform_node_inner(figma_node, &mut assets, false, None, None);
    DesignIR {
        version: "1.0".into(),
        name: name.into(),
        theme: None,
        components: vec![root],
        assets,
    }
}

fn transform_node_inner(
    figma: &FigmaNode,
    assets: &mut Vec<Asset>,
    is_overlay: bool,
    parent_layout_mode: Option<&LayoutMode>,
    parent_bb: Option<&BoundingBox>,
) -> Node {
    let node_type = classify_node(figma);

    // Determine this node's own auto-layout mode
    let has_auto_layout = figma
        .layout_mode
        .as_deref()
        .is_some_and(|m| m == "HORIZONTAL" || m == "VERTICAL" || m == "GRID");
    let my_layout_mode = if has_auto_layout {
        match figma.layout_mode.as_deref() {
            Some("HORIZONTAL") => Some(LayoutMode::Horizontal),
            Some("VERTICAL") => Some(LayoutMode::Vertical),
            Some("GRID") => Some(LayoutMode::Grid),
            _ => None,
        }
    } else {
        None
    };

    // Build children (filter hidden, skip masks)
    let parent_has_no_layout = !has_auto_layout;
    let children: Vec<Node> = figma
        .children
        .iter()
        .filter(|c| c.visible.unwrap_or(true))
        .filter(|c| !c.is_mask.unwrap_or(false))
        .enumerate()
        .map(|(i, c)| {
            let child_is_overlay = detect_figma_overlay(c, figma);
            let mut child = transform_node_inner(
                c,
                assets,
                child_is_overlay,
                my_layout_mode.as_ref(),
                figma.absolute_bounding_box.as_ref(),
            );
            // Set z-index for absolute children in no-layout parents
            if parent_has_no_layout
                && figma.absolute_bounding_box.is_some()
                && let Some(ref mut layout) = child.layout
                && layout.position.is_some()
            {
                layout.z_index = Some(i as i32);
            }
            child
        })
        .collect();

    // Text node
    if node_type == NodeType::Text {
        return make_text_node(figma, assets, parent_layout_mode, parent_bb);
    }

    // Image node (has image fill)
    if node_type == NodeType::Image {
        return make_image_node(figma, assets, parent_layout_mode, parent_bb);
    }

    // Wrapper flattening: single-child frame where child is not text
    if children.len() == 1
        && !has_auto_layout
        && children[0].node_type != NodeType::Text
        && figma.fills.is_empty()
        && figma.strokes.is_empty()
        && figma.effects.is_empty()
    {
        let mut child = children.into_iter().next().unwrap();
        // Transfer wrapper's absolute position to child if child doesn't have one
        let wrapper_layout = build_layout(figma, parent_layout_mode, parent_bb);
        if let Some(ref wpos) = wrapper_layout.position
            && let Some(ref mut cl) = child.layout
            && (cl.position.is_none()
                || cl
                    .position
                    .as_ref()
                    .is_some_and(|p| p.x.abs() < 0.01 && p.y.abs() < 0.01))
        {
            cl.position = Some(Position {
                x: wpos.x,
                y: wpos.y,
            });
        }
        child.overlay = child.overlay || is_overlay;
        return child;
    }

    // Same-direction flex chain collapse for multi-child when rest are vectors
    let children = if children.len() > 1
        && my_layout_mode.is_some()
        && children[1..]
            .iter()
            .all(|c| c.node_type == NodeType::Vector || c.node_type == NodeType::BooleanOp)
    {
        // Collapse: return first child only
        vec![children.into_iter().next().unwrap()]
    } else {
        children
    };

    // Icon container: recursively checks if all visible children are vectors/booleans
    // Export as a single SVG image.
    if is_icon_container(figma) && !children.is_empty() {
        assets.push(Asset {
            id: figma.id.clone(),
            name: figma.name.clone(),
            asset_type: AssetType::Svg,
            format: "svg".into(),
            data: String::new(),
            url: None,
            source_ref: None,
        });
        let layout = build_layout(figma, parent_layout_mode, parent_bb);
        let style = build_style(figma, assets);
        return Node {
            id: figma.id.clone(),
            name: figma.name.clone(),
            node_type: NodeType::Image,
            layout: Some(layout),
            style,
            text: None,
            vector: None,
            vector_paths: None,
            boolean_op: None,
            mask: build_mask(figma),
            component: build_component(figma),
            children: vec![],
            overlay: is_overlay,
        };
    }

    let layout = build_layout(figma, parent_layout_mode, parent_bb);
    let style = build_style(figma, assets);

    Node {
        id: figma.id.clone(),
        name: figma.name.clone(),
        node_type,
        layout: Some(layout),
        style,
        text: None,
        vector: None,
        vector_paths: None,
        boolean_op: None,
        mask: build_mask(figma),
        component: build_component(figma),
        children,
        overlay: is_overlay,
    }
}

fn classify_node(figma: &FigmaNode) -> NodeType {
    match figma.node_type.as_str() {
        "TEXT" => NodeType::Text,
        // Complex vector shapes → export as SVG
        "VECTOR" | "LINE" | "REGULAR_POLYGON" | "STAR" | "ELLIPSE" if figma.children.is_empty() => {
            NodeType::Image
        }
        // RECTANGLE: render as <div> with CSS (bg, border, radius) — no SVG needed.
        // Falls through to the Frame-like branch below.
        "BOOLEAN_OPERATION" => NodeType::BooleanOp,
        "INSTANCE" => NodeType::Instance,
        "GROUP" => NodeType::Group,
        _ => {
            // Frame-like (including RECTANGLE): check for image fill
            if has_image_fill(figma) {
                NodeType::Image
            } else {
                NodeType::Frame
            }
        }
    }
}

fn has_image_fill(figma: &FigmaNode) -> bool {
    figma
        .fills
        .iter()
        .any(|f| f.visible && f.paint_type == "IMAGE")
}

fn build_layout(
    figma: &FigmaNode,
    parent_flex_dir: Option<&LayoutMode>,
    parent_bb: Option<&BoundingBox>,
) -> Layout {
    let has_auto_layout = figma
        .layout_mode
        .as_deref()
        .is_some_and(|m| m == "HORIZONTAL" || m == "VERTICAL" || m == "GRID");

    let mode = if has_auto_layout {
        match figma.layout_mode.as_deref() {
            Some("HORIZONTAL") => Some(LayoutMode::Horizontal),
            Some("VERTICAL") => Some(LayoutMode::Vertical),
            Some("GRID") => Some(LayoutMode::Grid),
            _ => None,
        }
    } else {
        None
    };

    // Absolute positioning detection
    let is_absolute_positioned = figma
        .layout_positioning
        .as_deref()
        .is_some_and(|p| p == "ABSOLUTE");
    let parent_has_no_auto_layout = parent_flex_dir.is_none();
    let is_absolute = is_absolute_positioned || (parent_has_no_auto_layout && parent_bb.is_some());

    // Position
    let position = if is_absolute {
        figma.absolute_bounding_box.as_ref().and_then(|bb| {
            parent_bb.map(|pbb| Position {
                x: round_half_px(bb.x - pbb.x),
                y: round_half_px(bb.y - pbb.y),
            })
        })
    } else {
        None
    };

    // Dimensions
    let (width, height) = {
        let w = figma.layout_sizing_horizontal.as_deref().map(|s| match s {
            "FILL" => Dimension {
                dim_type: DimensionType::Fill,
                value: None,
            },
            "HUG" => Dimension {
                dim_type: DimensionType::Hug,
                value: None,
            },
            _ => Dimension {
                dim_type: DimensionType::Fixed,
                value: figma
                    .absolute_bounding_box
                    .as_ref()
                    .map(|bb| round_half_px(bb.width)),
            },
        });
        let h = figma.layout_sizing_vertical.as_deref().map(|s| match s {
            "FILL" => Dimension {
                dim_type: DimensionType::Fill,
                value: None,
            },
            "HUG" => Dimension {
                dim_type: DimensionType::Hug,
                value: None,
            },
            _ => Dimension {
                dim_type: DimensionType::Fixed,
                value: figma
                    .absolute_bounding_box
                    .as_ref()
                    .map(|bb| round_half_px(bb.height)),
            },
        });
        // Absolute nodes: FILL sizing doesn't apply (no parent layout to fill).
        // Fall back to bounding box for FILL or missing sizing.
        if is_absolute {
            let bb_w = || {
                figma.absolute_bounding_box.as_ref().map(|bb| Dimension {
                    dim_type: DimensionType::Fixed,
                    value: Some(round_half_px(bb.width)),
                })
            };
            let bb_h = || {
                figma.absolute_bounding_box.as_ref().map(|bb| Dimension {
                    dim_type: DimensionType::Fixed,
                    value: Some(round_half_px(bb.height)),
                })
            };
            let w = match w.as_ref().map(|d| &d.dim_type) {
                Some(DimensionType::Fill) | None => bb_w().or(w),
                _ => w,
            };
            let h = match h.as_ref().map(|d| &d.dim_type) {
                Some(DimensionType::Fill) | None => bb_h().or(h),
                _ => h,
            };
            (w, h)
        } else {
            (w, h)
        }
    };

    // Padding
    let padding = if has_auto_layout {
        let pt = round_half_px(figma.padding_top.unwrap_or(0.0));
        let pr = round_half_px(figma.padding_right.unwrap_or(0.0));
        let pb = round_half_px(figma.padding_bottom.unwrap_or(0.0));
        let pl = round_half_px(figma.padding_left.unwrap_or(0.0));
        if pt > 0.0 || pr > 0.0 || pb > 0.0 || pl > 0.0 {
            Some(Padding {
                top: pt,
                right: pr,
                bottom: pb,
                left: pl,
            })
        } else {
            None
        }
    } else {
        None
    };

    // Gap
    let gap = if has_auto_layout {
        figma.item_spacing.map(round_half_px).filter(|g| *g > 0.0)
    } else {
        None
    };

    // Alignment
    let main_axis_align = figma
        .primary_axis_align_items
        .as_deref()
        .and_then(map_alignment);
    let cross_axis_align = figma
        .counter_axis_align_items
        .as_deref()
        .and_then(map_alignment);

    // Overflow
    let overflow = if figma.clips_content == Some(true) {
        Some(Overflow::Hidden)
    } else {
        None
    };

    // Rotation (Figma stores radians, convert to degrees, no negation)
    let rotation = figma.rotation.and_then(|r| {
        if r.abs() < 0.001 {
            None
        } else {
            Some(round_decimal(r.to_degrees(), 2))
        }
    });

    // Detect flip from relativeTransform matrix (negative determinant = flipped)
    let (flip_x, flip_y) = figma
        .relative_transform
        .as_ref()
        .map(|m| {
            let t = decompose_matrix(m);
            (t.scale_x < 0.0, t.scale_y < 0.0)
        })
        .map(|(fx, fy)| {
            (
                if fx { Some(true) } else { None },
                if fy { Some(true) } else { None },
            )
        })
        .unwrap_or((None, None));

    // Flex wrap
    let wrap = figma.layout_wrap.as_deref().map(|w| w == "WRAP");

    let wrap_gap = figma.counter_axis_spacing.map(round_half_px);

    let wrap_align = figma
        .counter_axis_align_content
        .as_deref()
        .and_then(map_alignment);

    // Min/max constraints
    let min_width = figma.min_width.map(round_half_px);
    let max_width = figma.max_width.map(round_half_px);
    let min_height = figma.min_height.map(round_half_px);
    let max_height = figma.max_height.map(round_half_px);

    // Self alignment
    let self_align = figma.layout_align.as_deref().and_then(|a| match a {
        "STRETCH" => Some(Alignment::Stretch),
        "CENTER" => Some(Alignment::Center),
        "MIN" => Some(Alignment::Start),
        "MAX" => Some(Alignment::End),
        _ => None,
    });

    // Per-axis overflow
    let (overflow_x, overflow_y) = match figma.overflow_direction.as_deref() {
        Some("HORIZONTAL_SCROLLING") => (Some(Overflow::Scroll), None),
        Some("VERTICAL_SCROLLING") => (None, Some(Overflow::Scroll)),
        Some("HORIZONTAL_AND_VERTICAL_SCROLLING") => {
            (Some(Overflow::Scroll), Some(Overflow::Scroll))
        }
        _ => (None, None),
    };

    Layout {
        mode,
        width,
        height,
        padding,
        gap,
        main_axis_align,
        cross_axis_align,
        constraints: None,
        position,
        overflow,
        rotation,
        parent_flex_dir: parent_flex_dir.cloned(),
        wrap,
        wrap_gap,
        wrap_align,
        min_width,
        max_width,
        min_height,
        max_height,
        self_align,
        overflow_x,
        overflow_y,
        z_index: None,
        aspect_ratio: None,
        grid_columns_sizing: figma.grid_columns_sizing.clone(),
        grid_rows_sizing: figma.grid_rows_sizing.clone(),
        grid_column_gap: figma.grid_column_gap,
        grid_row_gap: figma.grid_row_gap,
        grid_column_span: figma.grid_column_span,
        grid_row_span: figma.grid_row_span,
        grid_column_start: figma.grid_column_anchor_index.map(|i| i + 1),
        grid_row_start: figma.grid_row_anchor_index.map(|i| i + 1),
        flip_x,
        flip_y,
    }
}

fn map_alignment(s: &str) -> Option<Alignment> {
    match s {
        "MIN" => Some(Alignment::Start),
        "CENTER" => Some(Alignment::Center),
        "MAX" => Some(Alignment::End),
        "SPACE_BETWEEN" => Some(Alignment::SpaceBetween),
        "STRETCH" => Some(Alignment::Stretch),
        _ => None,
    }
}

fn build_style(figma: &FigmaNode, assets: &mut Vec<Asset>) -> Option<Style> {
    let fills = transform_fills(&figma.fills, assets, &figma.id, &figma.name);
    let stroke = transform_stroke(figma, assets);
    let border_radius = transform_border_radius(figma);
    let effects = transform_effects(figma);
    let opacity = figma.opacity.filter(|o| (*o - 1.0).abs() > 0.01);

    if fills.is_none()
        && stroke.is_none()
        && border_radius.is_none()
        && effects.is_none()
        && opacity.is_none()
    {
        return None;
    }

    let blend_mode = parse_blend_mode(figma.blend_mode.as_deref());

    Some(Style {
        fills,
        stroke,
        border_radius,
        effects,
        opacity,
        blend_mode,
    })
}

fn transform_fills(
    fills: &[FigmaPaint],
    assets: &mut Vec<Asset>,
    node_id: &str,
    node_name: &str,
) -> Option<Vec<Fill>> {
    let result: Vec<Fill> = fills
        .iter()
        .filter(|f| f.visible)
        .filter_map(|f| transform_fill(f, assets, node_id, node_name))
        .collect();
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

fn transform_fill(
    paint: &FigmaPaint,
    assets: &mut Vec<Asset>,
    node_id: &str,
    node_name: &str,
) -> Option<Fill> {
    match paint.paint_type.as_str() {
        "SOLID" => {
            let color = resolve_paint_color(paint)?;
            // opacity already baked into color alpha by resolve_paint_color
            Some(Fill::Solid {
                color,
                opacity: None,
            })
        }
        "GRADIENT_LINEAR" | "GRADIENT_RADIAL" | "GRADIENT_ANGULAR" | "GRADIENT_DIAMOND" => {
            let gradient_type = match paint.paint_type.as_str() {
                "GRADIENT_LINEAR" => GradientType::Linear,
                "GRADIENT_RADIAL" | "GRADIENT_DIAMOND" => GradientType::Radial,
                _ => GradientType::Angular,
            };
            let paint_opacity = paint.opacity.unwrap_or(1.0);
            let stops: Vec<GradientStop> = paint
                .gradient_stops
                .as_ref()
                .map(|stops| {
                    stops
                        .iter()
                        .map(|s| {
                            // Bake paint-level opacity into each stop's alpha
                            let mut c = s.color.clone_for_hex();
                            c.a *= paint_opacity;
                            GradientStop {
                                position: s.position,
                                color: c.to_hex_string(),
                            }
                        })
                        .collect()
                })
                .unwrap_or_default();

            // Compute gradient angle from handle positions
            let angle = if gradient_type == GradientType::Linear {
                paint
                    .gradient_handle_positions
                    .as_ref()
                    .filter(|h| h.len() >= 2)
                    .map(|h| {
                        let dx = h[1].x - h[0].x;
                        let dy = h[1].y - h[0].y;
                        round_decimal(dy.atan2(dx).to_degrees(), 2)
                    })
            } else {
                None
            };

            Some(Fill::Gradient {
                gradient_type,
                stops,
                angle,
            })
        }
        "IMAGE" => {
            // Use the Figma node ID as asset ID — the export API needs node IDs, not imageRefs.
            let asset_id = node_id.to_string();
            if !assets.iter().any(|a| a.id == asset_id) {
                assets.push(Asset {
                    id: asset_id.clone(),
                    name: node_name.to_string(),
                    asset_type: AssetType::Image,
                    format: "png".into(),
                    data: String::new(),
                    url: None,
                    source_ref: paint.image_ref.clone(),
                });
            }
            Some(Fill::Image {
                asset_ref: asset_id,
                scale_mode: parse_scale_mode(paint.scale_mode.as_deref()),
            })
        }
        _ => None,
    }
}

fn resolve_paint_color(paint: &FigmaPaint) -> Option<String> {
    paint.color.as_ref().map(|c| {
        let mut color = c.clone_for_hex();
        // Apply paint-level opacity to alpha
        if let Some(op) = paint.opacity {
            color.a *= op;
        }
        color.to_hex_string()
    })
}

/// Helper trait to clone FigmaColor data for hex conversion with modified alpha
trait CloneForHex {
    fn clone_for_hex(&self) -> ColorData;
}

struct ColorData {
    r: f64,
    g: f64,
    b: f64,
    a: f64,
}

impl ColorData {
    fn to_hex_string(&self) -> String {
        let r = (self.r * 255.0).round() as u8;
        let g = (self.g * 255.0).round() as u8;
        let b = (self.b * 255.0).round() as u8;
        if (self.a - 1.0).abs() < 0.01 {
            format!("#{r:02X}{g:02X}{b:02X}")
        } else {
            let a = (self.a * 255.0).round() as u8;
            format!("#{r:02X}{g:02X}{b:02X}{a:02X}")
        }
    }
}

impl CloneForHex for crate::figma::types::FigmaColor {
    fn clone_for_hex(&self) -> ColorData {
        ColorData {
            r: self.r,
            g: self.g,
            b: self.b,
            a: self.a,
        }
    }
}

fn transform_stroke(figma: &FigmaNode, assets: &mut Vec<Asset>) -> Option<Stroke> {
    let visible_stroke = figma.strokes.iter().find(|s| s.visible)?;
    let color = resolve_paint_color(visible_stroke)?;
    let width = round_half_px(figma.stroke_weight.unwrap_or(1.0));

    let position = figma.stroke_align.as_deref().and_then(|a| match a {
        "INSIDE" => Some(StrokePosition::Inside),
        "OUTSIDE" => Some(StrokePosition::Outside),
        "CENTER" => Some(StrokePosition::Center),
        _ => None,
    });

    let side_widths = figma.individual_stroke_weights.as_ref().map(|w| {
        [
            round_half_px(w.top),
            round_half_px(w.right),
            round_half_px(w.bottom),
            round_half_px(w.left),
        ]
    });

    let dashed = figma.stroke_dashes.as_ref().map(|d| !d.is_empty());

    // Register stroke image assets if needed (rare but possible)
    let _ = assets;

    Some(Stroke {
        color,
        width,
        position,
        side_widths,
        dashed,
    })
}

fn transform_border_radius(figma: &FigmaNode) -> Option<BorderRadius> {
    if let Some(radii) = figma.rectangle_corner_radii {
        let [tl, tr, br, bl] = radii;
        Some(BorderRadius {
            top_left: round_half_px(tl),
            top_right: round_half_px(tr),
            bottom_right: round_half_px(br),
            bottom_left: round_half_px(bl),
        })
    } else {
        figma.corner_radius.map(|r| {
            let r = round_half_px(r);
            BorderRadius {
                top_left: r,
                top_right: r,
                bottom_right: r,
                bottom_left: r,
            }
        })
    }
}

fn transform_effects(figma: &FigmaNode) -> Option<Vec<Effect>> {
    let effects: Vec<Effect> = figma
        .effects
        .iter()
        .filter(|e| e.visible)
        .filter_map(|e| match e.effect_type.as_str() {
            "DROP_SHADOW" => {
                let color = e.color.as_ref().map(|c| c.to_hex()).unwrap_or_default();
                let offset = e
                    .offset
                    .as_ref()
                    .map(|o| Position {
                        x: round_half_px(o.x),
                        y: round_half_px(o.y),
                    })
                    .unwrap_or(Position { x: 0.0, y: 0.0 });
                Some(Effect::DropShadow {
                    offset,
                    radius: round_half_px(e.radius.unwrap_or(0.0)),
                    spread: e.spread.map(round_half_px),
                    color,
                })
            }
            "INNER_SHADOW" => {
                let color = e.color.as_ref().map(|c| c.to_hex()).unwrap_or_default();
                let offset = e
                    .offset
                    .as_ref()
                    .map(|o| Position {
                        x: round_half_px(o.x),
                        y: round_half_px(o.y),
                    })
                    .unwrap_or(Position { x: 0.0, y: 0.0 });
                Some(Effect::InnerShadow {
                    offset,
                    radius: round_half_px(e.radius.unwrap_or(0.0)),
                    spread: e.spread.map(round_half_px),
                    color,
                })
            }
            "LAYER_BLUR" => Some(Effect::Blur {
                blur_type: Some(crate::ir::schema::BlurType::Layer),
                radius: round_half_px(e.radius.unwrap_or(0.0)),
            }),
            "BACKGROUND_BLUR" => Some(Effect::Blur {
                blur_type: Some(crate::ir::schema::BlurType::Background),
                radius: round_half_px(e.radius.unwrap_or(0.0)),
            }),
            _ => None,
        })
        .collect();

    if effects.is_empty() {
        None
    } else {
        Some(effects)
    }
}

fn make_text_node(
    figma: &FigmaNode,
    assets: &mut Vec<Asset>,
    parent_layout_mode: Option<&LayoutMode>,
    parent_bb: Option<&BoundingBox>,
) -> Node {
    let layout = build_layout(figma, parent_layout_mode, parent_bb);
    let style_obj = build_style(figma, assets);

    // Resolve text color from style fills (character-level fills override node fills)
    let text_style = figma.style.as_ref();
    let fills_source = text_style
        .and_then(|s| s.fills.as_ref())
        .unwrap_or(&figma.fills);
    let text_color_fills: Vec<Fill> = fills_source
        .iter()
        .filter(|f| f.visible)
        .filter_map(|f| {
            if f.paint_type == "SOLID" {
                resolve_paint_color(f).map(|color| Fill::Solid {
                    color,
                    opacity: None,
                })
            } else {
                None
            }
        })
        .collect();

    // Text opacity from node opacity — round to 2 decimal places
    let text_opacity = figma
        .opacity
        .filter(|o| (*o - 1.0).abs() > 0.01)
        .map(|o| round_decimal(o, 2));

    let style = if !text_color_fills.is_empty() || text_opacity.is_some() {
        Some(Style {
            fills: if text_color_fills.is_empty() {
                None
            } else {
                Some(text_color_fills)
            },
            stroke: None,
            border_radius: None,
            effects: None,
            opacity: text_opacity,
            blend_mode: None,
        })
    } else {
        style_obj
    };

    let text_props = text_style.map(|ts| {
        let font_size = ts.font_size.map(round_half_px);
        let font_weight = ts.font_weight.map(|w| w as u32);
        let line_height = ts.line_height_px.and_then(|lh| {
            ts.font_size.map(|fs| {
                if fs > 0.0 {
                    round_decimal(lh / fs, 3)
                } else {
                    1.0
                }
            })
        });
        let letter_spacing = ts.letter_spacing.and_then(|ls| {
            ts.font_size.map(|fs| {
                if fs > 0.0 {
                    round_decimal(ls / fs, 4)
                } else {
                    0.0
                }
            })
        });

        let text_align = ts.text_align_horizontal.as_deref().and_then(|a| match a {
            "LEFT" => Some(TextAlign::Left),
            "CENTER" => Some(TextAlign::Center),
            "RIGHT" => Some(TextAlign::Right),
            "JUSTIFIED" => Some(TextAlign::Justify),
            _ => None,
        });

        let text_decoration = ts.text_decoration.as_deref().and_then(|d| match d {
            "UNDERLINE" => Some(TextDecoration::Underline),
            "STRIKETHROUGH" => Some(TextDecoration::Strikethrough),
            _ => None,
        });

        let text_transform = ts.text_case.as_deref().and_then(|c| match c {
            "UPPER" => Some(TextTransform::Uppercase),
            "LOWER" => Some(TextTransform::Lowercase),
            "TITLE" => Some(TextTransform::Capitalize),
            "SMALL_CAPS" => Some(TextTransform::SmallCaps),
            "SMALL_CAPS_FORCED" => Some(TextTransform::AllSmallCaps),
            _ => None,
        });

        let truncation = ts.text_truncation.as_deref().and_then(|t| match t {
            "ENDING" => Some(Truncation::Ellipsis),
            _ => None,
        });

        let italic = ts.italic.filter(|&i| i);

        let vertical_align = ts.text_align_vertical.as_deref().and_then(|a| match a {
            "TOP" => None,
            "CENTER" => Some(VerticalAlign::Center),
            "BOTTOM" => Some(VerticalAlign::Bottom),
            _ => None,
        });

        let paragraph_spacing = ts.paragraph_spacing.filter(|&ps| ps > 0.0);
        let max_lines = ts.max_lines;

        let hyperlink = ts
            .hyperlink
            .as_ref()
            .and_then(|h| h.get("url").and_then(|v| v.as_str()).map(|s| s.to_string()));

        // List type from lineTypes array (per-line list formatting)
        let list_type = figma.line_types.as_ref().and_then(|lt| {
            lt.first().and_then(|t| match t.as_str() {
                "UNORDERED" => Some(ListType::Unordered),
                "ORDERED" => Some(ListType::Ordered),
                _ => None,
            })
        });

        TextProps {
            content: figma.characters.clone().unwrap_or_default(),
            font_size,
            font_family: ts.font_family.clone(),
            font_weight,
            line_height,
            letter_spacing,
            text_align,
            text_decoration,
            text_transform,
            truncation,
            italic,
            vertical_align,
            paragraph_spacing,
            max_lines,
            hyperlink,
            list_type,
            opentype_flags: ts.opentype_flags.clone().filter(|m| !m.is_empty()),
            spans: build_text_spans(figma, ts),
        }
    });

    Node {
        id: figma.id.clone(),
        name: figma.name.clone(),
        node_type: NodeType::Text,
        layout: Some(layout),
        style,
        text: text_props,
        vector: None,
        vector_paths: None,
        boolean_op: None,
        mask: None,
        component: None,
        children: vec![],
        overlay: false,
    }
}

fn make_image_node(
    figma: &FigmaNode,
    assets: &mut Vec<Asset>,
    parent_layout_mode: Option<&LayoutMode>,
    parent_bb: Option<&BoundingBox>,
) -> Node {
    let mut layout = build_layout(figma, parent_layout_mode, parent_bb);

    // Compute aspect ratio from bounding box for images
    if let Some(ref bb) = figma.absolute_bounding_box
        && bb.width > 0.0
        && bb.height > 0.0
    {
        layout.aspect_ratio = Some(round_decimal(bb.width / bb.height, 3));
    }

    // Register image assets from fills OR export shape as SVG
    let fills = transform_fills(&figma.fills, assets, &figma.id, &figma.name);
    let has_image_asset = assets.iter().any(|a| a.id == figma.id);

    // Try to extract inline SVG paths from fillGeometry for simple vectors.
    // Avoids a remote SVG export round-trip — renders directly in JSX.
    let vector_paths = extract_vector_paths(figma);

    if !has_image_asset && vector_paths.is_none() {
        // Vector/shape node without image fill or inline paths → export as SVG
        let is_shape = matches!(
            figma.node_type.as_str(),
            "VECTOR"
                | "LINE"
                | "REGULAR_POLYGON"
                | "STAR"
                | "ELLIPSE"
                | "RECTANGLE"
                | "BOOLEAN_OPERATION"
        );
        let format = if is_shape { "svg" } else { "png" };
        assets.push(Asset {
            id: figma.id.clone(),
            name: figma.name.clone(),
            asset_type: if format == "svg" {
                AssetType::Svg
            } else {
                AssetType::Image
            },
            format: format.into(),
            data: String::new(),
            url: None,
            source_ref: None,
        });
    }
    let style = Some(Style {
        fills,
        stroke: None,
        border_radius: transform_border_radius(figma),
        effects: None,
        opacity: figma.opacity.filter(|o| (*o - 1.0).abs() > 0.01),
        blend_mode: parse_blend_mode(figma.blend_mode.as_deref()),
    });

    Node {
        id: figma.id.clone(),
        name: figma.name.clone(),
        node_type: NodeType::Image,
        layout: Some(layout),
        style,
        text: None,
        vector: None,
        vector_paths,
        boolean_op: None,
        mask: None,
        component: None,
        children: vec![],
        overlay: false,
    }
}

fn extract_vector_paths(figma: &FigmaNode) -> Option<Vec<crate::ir::schema::VectorPath>> {
    use crate::ir::schema::{FillRule, VectorPath};

    // Only extract for simple vector/shape nodes with fillGeometry
    let is_simple_vector = matches!(
        figma.node_type.as_str(),
        "VECTOR" | "LINE" | "REGULAR_POLYGON" | "STAR" | "ELLIPSE" | "BOOLEAN_OPERATION"
    );
    if !is_simple_vector {
        return None;
    }

    let geom = figma.fill_geometry.as_ref()?;
    if geom.is_empty() {
        return None;
    }

    // Resolve the primary fill color (first visible solid)
    let fill_color = figma
        .fills
        .iter()
        .find(|f| f.visible && f.paint_type == "SOLID")
        .and_then(resolve_paint_color);

    let paths: Vec<VectorPath> = geom
        .iter()
        .map(|p| {
            let fill_rule = p.winding_rule.as_deref().and_then(|r| match r {
                "NONZERO" => Some(FillRule::Nonzero),
                "EVENODD" => Some(FillRule::Evenodd),
                _ => None,
            });
            VectorPath {
                d: p.path.clone(),
                fill_rule,
                fill: fill_color.clone(),
                stroke: None,
                stroke_width: None,
            }
        })
        .collect();

    if paths.is_empty() { None } else { Some(paths) }
}

fn build_mask(figma: &FigmaNode) -> Option<Mask> {
    if figma.is_mask == Some(true) {
        Some(Mask {
            is_mask: true,
            mask_type: MaskType::Alpha,
        })
    } else {
        None
    }
}

fn build_component(figma: &FigmaNode) -> Option<ComponentInfo> {
    match figma.node_type.as_str() {
        "COMPONENT" | "COMPONENT_SET" => {
            let variants = figma
                .component_property_definitions
                .as_ref()
                .and_then(|defs| {
                    let map: HashMap<String, Vec<String>> = defs
                        .iter()
                        .filter(|(_, d)| d.prop_type == "VARIANT")
                        .filter_map(|(name, d)| {
                            d.variant_options
                                .as_ref()
                                .map(|opts| (name.clone(), opts.clone()))
                        })
                        .collect();
                    if map.is_empty() { None } else { Some(map) }
                });
            Some(ComponentInfo {
                is_component: true,
                variants,
                variant_values: None,
            })
        }
        "INSTANCE" => {
            let variant_values = figma.component_properties.as_ref().and_then(|props| {
                let map: HashMap<String, String> = props
                    .iter()
                    .filter(|(_, p)| p.prop_type == "VARIANT")
                    .filter_map(|(name, p)| p.value.as_str().map(|v| (name.clone(), v.to_string())))
                    .collect();
                if map.is_empty() { None } else { Some(map) }
            });
            Some(ComponentInfo {
                is_component: false,
                variants: None,
                variant_values,
            })
        }
        _ => None,
    }
}

fn is_icon_container(figma: &FigmaNode) -> bool {
    if figma.children.is_empty() {
        return false;
    }
    figma
        .children
        .iter()
        .filter(|c| c.visible.unwrap_or(true))
        .all(|c| {
            matches!(
                c.node_type.as_str(),
                "VECTOR" | "LINE" | "REGULAR_POLYGON" | "STAR" | "ELLIPSE" | "BOOLEAN_OPERATION"
            ) || is_icon_container(c)
        })
}

fn detect_figma_overlay(child: &FigmaNode, parent: &FigmaNode) -> bool {
    // Parent clips content
    if parent.clips_content != Some(true) {
        return false;
    }
    // Child has drop shadow
    let has_drop_shadow = child
        .effects
        .iter()
        .any(|e| e.visible && e.effect_type == "DROP_SHADOW");
    if !has_drop_shadow {
        return false;
    }
    // Child has solid background fill
    let has_solid_bg = child
        .fills
        .iter()
        .any(|f| f.visible && f.paint_type == "SOLID");
    if !has_solid_bg {
        return false;
    }
    // Child is > 400px (width or height)
    let is_large = child
        .absolute_bounding_box
        .as_ref()
        .is_some_and(|bb| bb.width > 400.0 || bb.height > 400.0);
    if !is_large {
        return false;
    }
    // Child is < 60% parent width
    let is_narrow = match (
        child.absolute_bounding_box.as_ref(),
        parent.absolute_bounding_box.as_ref(),
    ) {
        (Some(cbb), Some(pbb)) => pbb.width > 0.0 && cbb.width < pbb.width * 0.6,
        _ => false,
    };
    if !is_narrow {
        return false;
    }
    // Child has >= 3 children
    child.children.len() >= 3
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figma::types::{BoundingBox, FigmaColor, FigmaNode, FigmaPaint, FigmaTypeStyle};

    fn make_figma_node(name: &str, node_type: &str) -> FigmaNode {
        FigmaNode {
            id: format!("id-{name}"),
            name: name.into(),
            node_type: node_type.into(),
            visible: Some(true),
            children: vec![],
            layout_mode: None,
            layout_sizing_horizontal: None,
            layout_sizing_vertical: None,
            primary_axis_align_items: None,
            counter_axis_align_items: None,
            padding_left: None,
            padding_right: None,
            padding_top: None,
            padding_bottom: None,
            item_spacing: None,
            clips_content: None,
            layout_positioning: None,
            layout_wrap: None,
            counter_axis_spacing: None,
            counter_axis_align_content: None,
            min_width: None,
            max_width: None,
            min_height: None,
            max_height: None,
            layout_align: None,
            overflow_direction: None,
            grid_row_gap: None,
            grid_column_gap: None,
            grid_columns_sizing: None,
            grid_rows_sizing: None,
            grid_column_span: None,
            grid_row_span: None,
            grid_column_anchor_index: None,
            grid_row_anchor_index: None,
            absolute_bounding_box: None,
            fills: vec![],
            strokes: vec![],
            stroke_weight: None,
            individual_stroke_weights: None,
            stroke_align: None,
            stroke_dashes: None,
            effects: vec![],
            opacity: None,
            blend_mode: None,
            rotation: None,
            corner_radius: None,
            rectangle_corner_radii: None,
            fill_geometry: None,
            relative_transform: None,
            characters: None,
            style: None,
            line_types: None,
            character_style_overrides: None,
            style_override_table: None,
            is_mask: None,
            component_properties: None,
            component_property_definitions: None,
            bound_variables: None,
        }
    }

    #[test]
    fn test_figma_to_ir_basic() {
        let mut root = make_figma_node("TestFrame", "FRAME");
        root.layout_mode = Some("VERTICAL".into());
        root.layout_sizing_horizontal = Some("FIXED".into());
        root.layout_sizing_vertical = Some("HUG".into());
        root.absolute_bounding_box = Some(BoundingBox {
            x: 0.0,
            y: 0.0,
            width: 400.0,
            height: 300.0,
        });

        let ir = figma_to_ir("TestProject", &root);
        assert_eq!(ir.name, "TestProject");
        assert_eq!(ir.version, "1.0");
        assert_eq!(ir.components.len(), 1);
        assert_eq!(ir.components[0].name, "TestFrame");
        assert_eq!(ir.components[0].node_type, NodeType::Frame);
    }

    #[test]
    fn test_hidden_children_excluded() {
        let mut root = make_figma_node("Root", "FRAME");
        root.layout_mode = Some("VERTICAL".into());

        let mut visible_child = make_figma_node("Visible", "FRAME");
        visible_child.layout_mode = Some("HORIZONTAL".into());
        visible_child.visible = Some(true);

        let mut hidden_child = make_figma_node("Hidden", "FRAME");
        hidden_child.layout_mode = Some("HORIZONTAL".into());
        hidden_child.visible = Some(false);

        root.children = vec![visible_child, hidden_child];

        let ir = figma_to_ir("Test", &root);
        let root_node = &ir.components[0];
        // Only the visible child should be present
        assert_eq!(root_node.children.len(), 1);
        assert_eq!(root_node.children[0].name, "Visible");
    }

    #[test]
    fn test_text_node_transform() {
        let mut root = make_figma_node("Root", "FRAME");
        root.layout_mode = Some("VERTICAL".into());

        let mut text = make_figma_node("Label", "TEXT");
        text.characters = Some("Hello World".into());
        text.style = Some(FigmaTypeStyle {
            font_family: Some("Inter".into()),
            font_weight: Some(600.0),
            font_size: Some(16.0),
            line_height_px: Some(24.0),
            letter_spacing: None,
            text_align_horizontal: Some("CENTER".into()),
            text_decoration: None,
            text_case: None,
            text_truncation: None,
            italic: None,
            text_align_vertical: None,
            paragraph_spacing: None,
            max_lines: None,
            hyperlink: None,
            opentype_flags: None,
            fills: None,
            bound_variables: None,
        });

        root.children = vec![text];

        let ir = figma_to_ir("Test", &root);
        let text_node = &ir.components[0].children[0];
        assert_eq!(text_node.node_type, NodeType::Text);
        let tp = text_node.text.as_ref().unwrap();
        assert_eq!(tp.content, "Hello World");
        assert_eq!(tp.font_size, Some(16.0));
        assert_eq!(tp.font_weight, Some(600));
        assert_eq!(tp.font_family, Some("Inter".into()));
        assert_eq!(tp.text_align, Some(TextAlign::Center));
        // line_height = 24/16 = 1.5
        assert!((tp.line_height.unwrap() - 1.5).abs() < 0.001);
    }

    #[test]
    fn test_image_fill_creates_asset() {
        let mut node = make_figma_node("Hero", "FRAME");
        node.fills = vec![FigmaPaint {
            paint_type: "IMAGE".into(),
            visible: true,
            opacity: None,
            color: None,
            gradient_stops: None,
            gradient_handle_positions: None,
            image_ref: Some("abc123hash".into()),
            scale_mode: None,
            bound_variables: None,
        }];

        let ir = figma_to_ir("Test", &node);
        assert_eq!(ir.assets.len(), 1);
        assert_eq!(ir.assets[0].source_ref, Some("abc123hash".into()));
        assert_eq!(ir.components[0].node_type, NodeType::Image);
    }

    #[test]
    fn test_stroke_uses_visible() {
        let mut node = make_figma_node("Box", "FRAME");
        node.strokes = vec![
            FigmaPaint {
                paint_type: "SOLID".into(),
                visible: false,
                opacity: None,
                color: Some(FigmaColor {
                    r: 1.0,
                    g: 0.0,
                    b: 0.0,
                    a: 1.0,
                }),
                gradient_stops: None,
                gradient_handle_positions: None,
                image_ref: None,
                scale_mode: None,
                bound_variables: None,
            },
            FigmaPaint {
                paint_type: "SOLID".into(),
                visible: true,
                opacity: None,
                color: Some(FigmaColor {
                    r: 0.0,
                    g: 0.0,
                    b: 1.0,
                    a: 1.0,
                }),
                gradient_stops: None,
                gradient_handle_positions: None,
                image_ref: None,
                scale_mode: None,
                bound_variables: None,
            },
        ];
        node.stroke_weight = Some(2.0);

        let ir = figma_to_ir("Test", &node);
        let style = ir.components[0].style.as_ref().unwrap();
        let stroke = style.stroke.as_ref().unwrap();
        // Should use the visible (blue) stroke, not the hidden (red) one
        assert_eq!(stroke.color, "#0000FF");
        assert_eq!(stroke.width, 2.0);
    }

    #[test]
    fn test_icon_container_detection() {
        let mut container = make_figma_node("Icon", "FRAME");
        let vec1 = make_figma_node("Path1", "VECTOR");
        let vec2 = make_figma_node("Path2", "VECTOR");
        container.children = vec![vec1, vec2];

        assert!(is_icon_container(&container));

        // Not an icon container if there's a non-vector child
        let mut mixed = make_figma_node("Mixed", "FRAME");
        let frame_child = make_figma_node("Inner", "FRAME");
        let vec3 = make_figma_node("Path3", "VECTOR");
        mixed.children = vec![frame_child, vec3];

        assert!(!is_icon_container(&mixed));
    }

    #[test]
    fn test_round_half_px() {
        assert_eq!(round_half_px(10.0), 10.0);
        assert_eq!(round_half_px(10.3), 10.5);
        assert_eq!(round_half_px(10.7), 10.5);
        assert_eq!(round_half_px(10.8), 11.0);
        assert_eq!(round_half_px(10.25), 10.5);
        assert_eq!(round_half_px(10.75), 11.0);
    }

    #[test]
    fn test_round_decimal() {
        assert_eq!(round_decimal(1.5555, 2), 1.56);
        assert_eq!(round_decimal(1.5555, 3), 1.556);
        assert_eq!(round_decimal(1.0, 2), 1.0);
    }
}
