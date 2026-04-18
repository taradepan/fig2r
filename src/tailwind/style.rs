use crate::ir::schema::{
    BlendMode, BlurType, BorderRadius, Effect, Fill, GradientType, Stroke, StrokePosition,
};
use crate::tailwind::values;
use crate::warning::WarningCollector;
use std::collections::HashMap;

pub fn fill_classes(
    fills: &[Fill],
    theme_colors: Option<&HashMap<String, String>>,
    warnings: &mut WarningCollector,
    node_id: &str,
    node_name: &str,
) -> Vec<String> {
    let mut classes = Vec::new();
    for fill in fills {
        match fill {
            Fill::Solid { color, opacity } => {
                let color_class = if let Some(colors) = theme_colors {
                    colors
                        .iter()
                        .find(|(_, v)| v.eq_ignore_ascii_case(color))
                        .map(|(name, _)| format!("bg-{name}"))
                } else {
                    None
                };
                classes.push(color_class.unwrap_or_else(|| format!("bg-[{color}]")));
                if let Some(op) = opacity
                    && (*op - 1.0).abs() > 0.01
                {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    let pct = (op * 100.0).round() as u32;
                    classes.push(format!("bg-opacity-{pct}"));
                }
            }
            Fill::Gradient {
                gradient_type,
                stops,
                angle,
            } => match gradient_type {
                GradientType::Linear => {
                    // Check if we can use standard Tailwind gradient classes
                    let standard_dir = match angle {
                        Some(a) if (*a - 0.0).abs() < 22.5 => Some("bg-gradient-to-r"),
                        Some(a) if (*a - 45.0).abs() < 22.5 => Some("bg-gradient-to-br"),
                        Some(a) if (*a - 90.0).abs() < 22.5 => Some("bg-gradient-to-b"),
                        Some(a) if (*a - 135.0).abs() < 22.5 || (*a + 225.0).abs() < 22.5 => {
                            Some("bg-gradient-to-bl")
                        }
                        Some(a) if a.abs() > 157.5 => Some("bg-gradient-to-l"),
                        Some(a) if (*a + 135.0).abs() < 22.5 => Some("bg-gradient-to-tl"),
                        Some(a) if (*a + 90.0).abs() < 22.5 => Some("bg-gradient-to-t"),
                        Some(a) if (*a + 45.0).abs() < 22.5 => Some("bg-gradient-to-tr"),
                        None => Some("bg-gradient-to-r"),
                        _ => None,
                    };
                    let stops_uniform = stops.len() <= 3
                        && stops.first().is_none_or(|s| s.position < 0.01)
                        && stops.last().is_none_or(|s| (s.position - 1.0).abs() < 0.01)
                        && (stops.len() != 3 || (stops[1].position - 0.5).abs() < 0.05);

                    if let Some(dir) = standard_dir
                        && stops_uniform
                    {
                        // Standard Tailwind gradient
                        classes.push(dir.into());
                        if let Some(first) = stops.first() {
                            classes.push(format!("from-[{}]", first.color));
                        }
                        if stops.len() == 3 {
                            classes.push(format!("via-[{}]", stops[1].color));
                        }
                        if let Some(last) = stops.last()
                            && stops.len() > 1
                        {
                            classes.push(format!("to-[{}]", last.color));
                        }
                    } else {
                        // Arbitrary CSS for non-standard angle or stop positions
                        let deg = angle.unwrap_or(0.0).round();
                        let stop_str: Vec<String> = stops
                            .iter()
                            .map(|s| {
                                let pct = (s.position * 100.0).round();
                                format!("{}_{}%", s.color, pct)
                            })
                            .collect();
                        classes.push(format!(
                            "bg-[linear-gradient({}deg,{})]",
                            deg,
                            stop_str.join(",")
                        ));
                    }
                }
                GradientType::Radial => {
                    // Inline the full radial-gradient as an arbitrary bg value.
                    // Using underscores for spaces per Tailwind arbitrary value syntax.
                    let stop_str: Vec<String> = stops
                        .iter()
                        .map(|s| {
                            let pct = (s.position * 100.0).round();
                            format!("{}_{}%", s.color, pct)
                        })
                        .collect();
                    classes.push(format!(
                        "bg-[radial-gradient(circle,{})]",
                        stop_str.join(",")
                    ));
                }
                GradientType::Angular => {
                    warnings.warn(
                        node_id,
                        node_name,
                        "angular gradient converted to linear approximation",
                    );
                    classes.push("bg-gradient-to-r".into());
                    if let Some(first) = stops.first() {
                        classes.push(format!("from-[{}]", first.color));
                    }
                    if let Some(last) = stops.last()
                        && stops.len() > 1
                    {
                        classes.push(format!("to-[{}]", last.color));
                    }
                }
            },
            Fill::Image { .. } => {}
        }
    }
    classes
}

pub fn border_radius_classes(br: &BorderRadius) -> Vec<String> {
    let all_same = (br.top_left - br.top_right).abs() < 0.01
        && (br.top_right - br.bottom_right).abs() < 0.01
        && (br.bottom_right - br.bottom_left).abs() < 0.01;

    if all_same {
        return vec![values::border_radius_class(br.top_left)];
    }

    vec![
        corner_radius_class("tl", br.top_left),
        corner_radius_class("tr", br.top_right),
        corner_radius_class("br", br.bottom_right),
        corner_radius_class("bl", br.bottom_left),
    ]
}

fn corner_radius_class(corner: &str, px: f64) -> String {
    let base = values::border_radius_class(px);
    if let Some(suffix) = base.strip_prefix("rounded-") {
        format!("rounded-{corner}-{suffix}")
    } else {
        base
    }
}

pub fn stroke_classes(
    stroke: &Stroke,
    theme_colors: Option<&HashMap<String, String>>,
) -> Vec<String> {
    let mut classes = Vec::new();

    // strokeAlign: INSIDE → border (CSS default), OUTSIDE → outline, CENTER → outline with negative offset.
    // Per-side strokes always use border (outline can't do per-side).
    let use_outline =
        stroke.side_widths.is_none() && matches!(stroke.position, Some(StrokePosition::Outside));
    let center_align =
        stroke.side_widths.is_none() && matches!(stroke.position, Some(StrokePosition::Center));

    if let Some(sides) = &stroke.side_widths {
        let [top, right, bottom, left] = *sides;
        if top > 0.0 {
            classes.push(border_width_class("border-t", top));
        }
        if right > 0.0 {
            classes.push(border_width_class("border-r", right));
        }
        if bottom > 0.0 {
            classes.push(border_width_class("border-b", bottom));
        }
        if left > 0.0 {
            classes.push(border_width_class("border-l", left));
        }
    } else if use_outline || center_align {
        classes.push(outline_width_class(stroke.width));
    } else {
        classes.push(border_width_class("border", stroke.width));
    }

    if center_align {
        // Pull outline half the width so it straddles the box edge
        let offset = stroke.width / 2.0;
        classes.push(format!("outline-offset-[-{offset}px]"));
    }

    if stroke.dashed == Some(true) {
        if use_outline || center_align {
            classes.push("outline-dashed".into());
        } else {
            classes.push("border-dashed".into());
        }
    }

    let prefix = if use_outline || center_align {
        "outline"
    } else {
        "border"
    };
    let color_class = if let Some(colors) = theme_colors {
        colors
            .iter()
            .find(|(_, v)| v.eq_ignore_ascii_case(&stroke.color))
            .map(|(name, _)| format!("{prefix}-{name}"))
    } else {
        None
    };
    classes.push(color_class.unwrap_or_else(|| format!("{prefix}-[{}]", stroke.color)));

    classes
}

fn outline_width_class(width: f64) -> String {
    if (width - 1.0).abs() < 0.01 {
        "outline".into()
    } else if (width - 2.0).abs() < 0.01 {
        "outline-2".into()
    } else if (width - 4.0).abs() < 0.01 {
        "outline-4".into()
    } else if (width - 8.0).abs() < 0.01 {
        "outline-8".into()
    } else if width < 0.01 {
        "outline-0".into()
    } else {
        format!("outline-[{width}px]")
    }
}

fn border_width_class(prefix: &str, width: f64) -> String {
    // Exact pixel values — no rounding to preserve Figma fidelity
    if (width - 1.0).abs() < 0.01 {
        prefix.to_string()
    } else if (width - 2.0).abs() < 0.01 {
        format!("{prefix}-2")
    } else if (width - 4.0).abs() < 0.01 {
        format!("{prefix}-4")
    } else if (width - 8.0).abs() < 0.01 {
        format!("{prefix}-8")
    } else if width < 0.01 {
        format!("{prefix}-0")
    } else {
        format!("{prefix}-[{width}px]")
    }
}

pub fn effect_classes(effects: &[Effect], _warnings: &mut WarningCollector) -> Vec<String> {
    let mut classes = Vec::new();

    // Collect all shadow effects into a single comma-joined `shadow-[...]` class.
    // Tailwind merges classes by utility, so multiple `shadow-*` on one element
    // collapse to the last — we have to stack them into one CSS `box-shadow` list.
    let mut shadows: Vec<String> = Vec::new();
    for effect in effects {
        match effect {
            Effect::DropShadow {
                offset,
                radius,
                spread,
                color,
            } => shadows.push(shadow_body(
                offset.x,
                offset.y,
                *radius,
                spread.unwrap_or(0.0),
                color,
                false,
            )),
            Effect::InnerShadow {
                offset,
                radius,
                spread,
                color,
            } => shadows.push(shadow_body(
                offset.x,
                offset.y,
                *radius,
                spread.unwrap_or(0.0),
                color,
                true,
            )),
            Effect::Blur { .. } => {}
        }
    }
    if !shadows.is_empty() {
        classes.push(format!("shadow-[{}]", shadows.join(",")));
    }

    // Blurs are filter-based and don't need stacking like box-shadow.
    for effect in effects {
        if let Effect::Blur { blur_type, radius } = effect {
            match blur_type.as_ref().unwrap_or(&BlurType::Layer) {
                BlurType::Layer => classes.push(format!("blur-[{radius}px]")),
                BlurType::Background => classes.push(format!("backdrop-blur-[{radius}px]")),
            }
        }
    }

    classes
}

fn shadow_body(x: f64, y: f64, radius: f64, spread: f64, color: &str, inset: bool) -> String {
    // Use underscores for spaces per Tailwind arbitrary value syntax; round to
    // half-pixel precision for stable output.
    let x = (x * 2.0).round() / 2.0;
    let y = (y * 2.0).round() / 2.0;
    let radius = (radius * 2.0).round() / 2.0;
    let spread = (spread * 2.0).round() / 2.0;
    if inset {
        format!("inset_{x}px_{y}px_{radius}px_{spread}px_{color}")
    } else {
        format!("{x}px_{y}px_{radius}px_{spread}px_{color}")
    }
}

pub fn opacity_class(value: f64) -> String {
    values::opacity_class(value)
}

pub fn blend_mode_class(
    mode: &BlendMode,
    _node_id: &str,
    _node_name: &str,
    _warnings: &mut WarningCollector,
) -> Option<String> {
    let cls = match mode {
        BlendMode::Normal => return None,
        BlendMode::Multiply => "mix-blend-multiply",
        BlendMode::Screen => "mix-blend-screen",
        BlendMode::Overlay => "mix-blend-overlay",
        BlendMode::Darken => "mix-blend-darken",
        BlendMode::Lighten => "mix-blend-lighten",
        BlendMode::ColorDodge => "mix-blend-color-dodge",
        BlendMode::ColorBurn => "mix-blend-color-burn",
        BlendMode::HardLight => "mix-blend-hard-light",
        BlendMode::SoftLight => "mix-blend-soft-light",
        BlendMode::Difference => "mix-blend-difference",
        BlendMode::Exclusion => "mix-blend-exclusion",
        BlendMode::Hue => "mix-blend-hue",
        BlendMode::Saturation => "mix-blend-saturation",
        BlendMode::Color => "mix-blend-color",
        BlendMode::Luminosity => "mix-blend-luminosity",
        BlendMode::Unknown => return None,
    };
    Some(cls.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::schema::{GradientStop, Position};
    use crate::warning::WarningCollector;

    #[test]
    fn test_solid_fill() {
        let mut warnings = WarningCollector::new();
        let fills = vec![Fill::Solid {
            color: "#3B82F6".into(),
            opacity: None,
        }];
        let classes = fill_classes(&fills, None, &mut warnings, "test", "Test");
        assert!(classes.contains(&"bg-[#3B82F6]".to_string()));
    }

    #[test]
    fn test_solid_fill_with_theme() {
        let mut warnings = WarningCollector::new();
        let mut colors = std::collections::HashMap::new();
        colors.insert("primary".to_string(), "#3B82F6".to_string());
        let fills = vec![Fill::Solid {
            color: "#3B82F6".into(),
            opacity: None,
        }];
        let classes = fill_classes(&fills, Some(&colors), &mut warnings, "test", "Test");
        assert!(classes.contains(&"bg-primary".to_string()));
    }

    #[test]
    fn test_linear_gradient() {
        let mut warnings = WarningCollector::new();
        let fills = vec![Fill::Gradient {
            gradient_type: GradientType::Linear,
            stops: vec![
                GradientStop {
                    position: 0.0,
                    color: "#3B82F6".into(),
                },
                GradientStop {
                    position: 1.0,
                    color: "#8B5CF6".into(),
                },
            ],
            angle: None,
        }];
        let classes = fill_classes(&fills, None, &mut warnings, "test", "Test");
        assert!(classes.contains(&"bg-gradient-to-r".to_string()));
        assert!(classes.contains(&"from-[#3B82F6]".to_string()));
        assert!(classes.contains(&"to-[#8B5CF6]".to_string()));
    }

    #[test]
    fn test_angular_gradient_warns() {
        let mut warnings = WarningCollector::new();
        let fills = vec![Fill::Gradient {
            gradient_type: GradientType::Angular,
            stops: vec![
                GradientStop {
                    position: 0.0,
                    color: "#000".into(),
                },
                GradientStop {
                    position: 1.0,
                    color: "#fff".into(),
                },
            ],
            angle: None,
        }];
        fill_classes(&fills, None, &mut warnings, "test", "Test");
        assert!(warnings.has_warnings());
    }

    #[test]
    fn test_border_radius_uniform() {
        let br = BorderRadius {
            top_left: 8.0,
            top_right: 8.0,
            bottom_right: 8.0,
            bottom_left: 8.0,
        };
        let classes = border_radius_classes(&br);
        assert_eq!(classes, vec!["rounded-[8px]"]);
    }

    #[test]
    fn test_border_radius_mixed() {
        let br = BorderRadius {
            top_left: 8.0,
            top_right: 8.0,
            bottom_right: 0.0,
            bottom_left: 0.0,
        };
        let classes = border_radius_classes(&br);
        assert!(classes.contains(&"rounded-tl-[8px]".to_string()));
        assert!(classes.contains(&"rounded-tr-[8px]".to_string()));
        assert!(classes.contains(&"rounded-br-none".to_string()));
        assert!(classes.contains(&"rounded-bl-none".to_string()));
    }

    #[test]
    fn test_stroke() {
        let stroke = Stroke {
            color: "#E5E7EB".into(),
            width: 1.0,
            position: None,
            side_widths: None,
            dashed: None,
        };
        let classes = stroke_classes(&stroke, None);
        assert!(classes.contains(&"border".to_string()));
        assert!(classes.contains(&"border-[#E5E7EB]".to_string()));
    }

    #[test]
    fn test_stroke_width_2() {
        let stroke = Stroke {
            color: "#000".into(),
            width: 2.0,
            position: None,
            side_widths: None,
            dashed: None,
        };
        let classes = stroke_classes(&stroke, None);
        assert!(classes.contains(&"border-2".to_string()));
    }

    #[test]
    fn test_drop_shadow() {
        let mut warnings = WarningCollector::new();
        let effects = vec![Effect::DropShadow {
            offset: Position { x: 0.0, y: 1.0 },
            radius: 2.0,
            spread: Some(0.0),
            color: "rgba(0,0,0,0.05)".into(),
        }];
        let classes = effect_classes(&effects, &mut warnings);
        assert!(classes[0].starts_with("shadow-["));
    }

    #[test]
    fn test_multi_drop_shadow_combined() {
        let mut warnings = WarningCollector::new();
        let effects = vec![
            Effect::DropShadow {
                offset: Position { x: 0.0, y: 1.0 },
                radius: 2.0,
                spread: Some(0.0),
                color: "rgba(0,0,0,0.05)".into(),
            },
            Effect::DropShadow {
                offset: Position { x: 0.0, y: 4.0 },
                radius: 16.0,
                spread: Some(0.0),
                color: "rgba(0,0,0,0.1)".into(),
            },
        ];
        let classes = effect_classes(&effects, &mut warnings);
        // Exactly one shadow-[...] class — the two shadows must be comma-joined.
        let shadow_classes: Vec<&String> = classes
            .iter()
            .filter(|c| c.starts_with("shadow-["))
            .collect();
        assert_eq!(
            shadow_classes.len(),
            1,
            "expected one combined shadow class"
        );
        let c = shadow_classes[0];
        assert!(c.contains("0px_1px_2px_0px_rgba(0,0,0,0.05)"));
        assert!(c.contains("0px_4px_16px_0px_rgba(0,0,0,0.1)"));
        // Comma separator between the two shadow bodies, no spaces inside brackets.
        assert!(c.contains("),0px_"));
        assert!(!c.contains(' '));
    }

    #[test]
    fn test_inner_and_drop_shadow_combined() {
        let mut warnings = WarningCollector::new();
        let effects = vec![
            Effect::DropShadow {
                offset: Position { x: 0.0, y: 2.0 },
                radius: 4.0,
                spread: None,
                color: "#000".into(),
            },
            Effect::InnerShadow {
                offset: Position { x: 0.0, y: 1.0 },
                radius: 2.0,
                spread: None,
                color: "#111".into(),
            },
        ];
        let classes = effect_classes(&effects, &mut warnings);
        let shadow_classes: Vec<&String> = classes
            .iter()
            .filter(|c| c.starts_with("shadow-["))
            .collect();
        assert_eq!(shadow_classes.len(), 1);
        let c = shadow_classes[0];
        assert!(c.contains("inset_"));
        // Drop shadow should come first (same relative order as in effects slice).
        let drop_idx = c.find("0px_2px_4px_0px_#000").unwrap();
        let inner_idx = c.find("inset_0px_1px_2px_0px_#111").unwrap();
        assert!(drop_idx < inner_idx);
    }

    #[test]
    fn test_blur_layer() {
        let mut warnings = WarningCollector::new();
        let effects = vec![Effect::Blur {
            blur_type: Some(BlurType::Layer),
            radius: 8.0,
        }];
        let classes = effect_classes(&effects, &mut warnings);
        assert!(classes.contains(&"blur-[8px]".to_string()));
    }

    #[test]
    fn test_blur_background() {
        let mut warnings = WarningCollector::new();
        let effects = vec![Effect::Blur {
            blur_type: Some(BlurType::Background),
            radius: 8.0,
        }];
        let classes = effect_classes(&effects, &mut warnings);
        assert!(classes.contains(&"backdrop-blur-[8px]".to_string()));
    }

    #[test]
    fn test_opacity() {
        let classes = opacity_class(0.5);
        assert_eq!(classes, "opacity-50");
    }

    #[test]
    fn test_blend_mode_emits_class() {
        let mut warnings = WarningCollector::new();
        let cls = blend_mode_class(&BlendMode::Multiply, "test-id", "TestNode", &mut warnings);
        assert_eq!(cls, Some("mix-blend-multiply".into()));
        assert!(!warnings.has_warnings());
    }

    #[test]
    fn test_blend_mode_normal_returns_none() {
        let mut warnings = WarningCollector::new();
        let cls = blend_mode_class(&BlendMode::Normal, "test-id", "TestNode", &mut warnings);
        assert_eq!(cls, None);
    }
}
