use crate::ir::schema::{Alignment, DimensionType, Layout, LayoutMode, Overflow};
use crate::tailwind::values::{dimension_class, spacing_class};

/// Build Tailwind grid-cols/grid-rows class from Figma sizing array.
/// Figma sends e.g. ["1FR", "200", "MIN_CONTENT"] — map to CSS values.
fn grid_template_class(prefix: &str, sizes: &[String]) -> String {
    // If all are "1FR" and count fits, use shorthand "grid-cols-N"
    if !sizes.is_empty() && sizes.iter().all(|s| s.eq_ignore_ascii_case("1FR")) {
        return format!("{prefix}-{}", sizes.len());
    }
    let parts: Vec<String> = sizes
        .iter()
        .map(|s| {
            let up = s.to_ascii_uppercase();
            match up.as_str() {
                "MIN_CONTENT" => "min-content".into(),
                "MAX_CONTENT" => "max-content".into(),
                "AUTO" => "auto".into(),
                _ if up.ends_with("FR") => {
                    let n = up.trim_end_matches("FR").trim();
                    format!("{n}fr")
                }
                _ => {
                    // Numeric → px
                    if let Ok(n) = s.parse::<f64>() {
                        format!("{n}px")
                    } else {
                        s.clone()
                    }
                }
            }
        })
        .collect();
    format!("{prefix}-[{}]", parts.join("_"))
}

fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a.max(1)
}

pub fn layout_classes(layout: &Layout) -> Vec<String> {
    let mut classes = Vec::new();

    match &layout.mode {
        Some(LayoutMode::Horizontal) => {
            classes.push("flex".into());
            classes.push("flex-row".into());
        }
        Some(LayoutMode::Vertical) => {
            classes.push("flex".into());
            classes.push("flex-col".into());
        }
        Some(LayoutMode::Grid) => {
            classes.push("grid".into());
            if let Some(ref cols) = layout.grid_columns_sizing {
                classes.push(grid_template_class("grid-cols", cols));
            }
            if let Some(ref rows) = layout.grid_rows_sizing {
                classes.push(grid_template_class("grid-rows", rows));
            }
            if let Some(g) = layout.grid_column_gap
                && g > 0.0
            {
                classes.push(spacing_class("gap-x", g));
            }
            if let Some(g) = layout.grid_row_gap
                && g > 0.0
            {
                classes.push(spacing_class("gap-y", g));
            }
        }
        _ => {}
    }

    // Grid child positioning
    if let Some(span) = layout.grid_column_span
        && span > 1
    {
        classes.push(format!("col-span-{span}"));
    }
    if let Some(span) = layout.grid_row_span
        && span > 1
    {
        classes.push(format!("row-span-{span}"));
    }
    if let Some(start) = layout.grid_column_start {
        classes.push(format!("col-start-{start}"));
    }
    if let Some(start) = layout.grid_row_start {
        classes.push(format!("row-start-{start}"));
    }

    if layout.wrap == Some(true) {
        classes.push("flex-wrap".into());
    }

    if let Some(gap) = layout.gap
        && gap > 0.0
    {
        classes.push(spacing_class("gap", gap));
    }

    if let Some(wg) = layout.wrap_gap
        && wg > 0.0
    {
        classes.push(spacing_class("gap-y", wg));
    }

    if let Some(ref align) = layout.wrap_align {
        let cls = match align {
            Alignment::SpaceBetween => Some("content-between"),
            Alignment::Center => Some("content-center"),
            Alignment::End => Some("content-end"),
            _ => None,
        };
        if let Some(c) = cls {
            classes.push(c.into());
        }
    }

    if let Some(ref pad) = layout.padding {
        let all_same = (pad.top - pad.bottom).abs() < 0.01
            && (pad.left - pad.right).abs() < 0.01
            && (pad.top - pad.left).abs() < 0.01;
        let x_same = (pad.left - pad.right).abs() < 0.01;
        let y_same = (pad.top - pad.bottom).abs() < 0.01;

        if all_same && pad.top > 0.0 {
            classes.push(spacing_class("p", pad.top));
        } else if x_same && y_same {
            if pad.left > 0.0 {
                classes.push(spacing_class("px", pad.left));
            }
            if pad.top > 0.0 {
                classes.push(spacing_class("py", pad.top));
            }
        } else {
            if pad.top > 0.0 {
                classes.push(spacing_class("pt", pad.top));
            }
            if pad.right > 0.0 {
                classes.push(spacing_class("pr", pad.right));
            }
            if pad.bottom > 0.0 {
                classes.push(spacing_class("pb", pad.bottom));
            }
            if pad.left > 0.0 {
                classes.push(spacing_class("pl", pad.left));
            }
        }
    }

    if let Some(ref align) = layout.main_axis_align {
        let cls = match align {
            Alignment::Start => "justify-start",
            Alignment::Center => "justify-center",
            Alignment::End => "justify-end",
            Alignment::SpaceBetween => "justify-between",
            Alignment::Stretch => "justify-stretch",
        };
        classes.push(cls.into());
    }

    if let Some(ref align) = layout.cross_axis_align {
        let cls = match align {
            Alignment::Start => "items-start",
            Alignment::Center => "items-center",
            Alignment::End => "items-end",
            Alignment::Stretch => "items-stretch",
            // SpaceBetween is only meaningful on main axis; on cross-axis, treat as start
            Alignment::SpaceBetween => "items-start",
        };
        classes.push(cls.into());
    }

    classes.extend(size_classes(layout));

    if let Some(ref pos) = layout.position {
        classes.push("absolute".into());
        classes.push(format!("top-[{}px]", pos.y));
        classes.push(format!("left-[{}px]", pos.x));
    }

    if let Some(rotation) = layout.rotation {
        classes.push(format!("rotate-[{rotation:.2}deg]"));
    }

    if layout.flip_x == Some(true) {
        classes.push("scale-x-[-1]".into());
    }
    if layout.flip_y == Some(true) {
        classes.push("scale-y-[-1]".into());
    }

    if let Some(z) = layout.z_index {
        classes.push(format!("z-[{z}]"));
    }

    if let Some(ar) = layout.aspect_ratio {
        if (ar - 1.0).abs() < 0.01 {
            classes.push("aspect-square".into());
        } else if (ar - 16.0 / 9.0).abs() < 0.05 {
            classes.push("aspect-video".into());
        } else {
            // Express as simplified ratio
            let w = (ar * 100.0).round() as u32;
            let h = 100u32;
            let g = gcd(w, h);
            classes.push(format!("aspect-[{}/{}]", w / g, h / g));
        }
    }

    // Per-axis overflow takes priority over simple overflow
    if layout.overflow_x.is_some() || layout.overflow_y.is_some() {
        if let Some(ref ox) = layout.overflow_x {
            match ox {
                Overflow::Hidden => classes.push("overflow-x-hidden".into()),
                Overflow::Scroll => classes.push("overflow-x-auto".into()),
                Overflow::Visible => {}
            }
        }
        if let Some(ref oy) = layout.overflow_y {
            match oy {
                Overflow::Hidden => classes.push("overflow-y-hidden".into()),
                Overflow::Scroll => classes.push("overflow-y-auto".into()),
                Overflow::Visible => {}
            }
        }
    } else if let Some(ref overflow) = layout.overflow {
        match overflow {
            Overflow::Hidden => classes.push("overflow-hidden".into()),
            Overflow::Scroll => classes.push("overflow-auto".into()),
            Overflow::Visible => {}
        }
    }

    classes
}

pub fn size_classes(layout: &Layout) -> Vec<String> {
    let mut classes = Vec::new();
    let in_flex = layout.parent_flex_dir.is_some();
    let parent_is_row = matches!(layout.parent_flex_dir, Some(LayoutMode::Horizontal));
    let parent_is_col = matches!(layout.parent_flex_dir, Some(LayoutMode::Vertical));

    let mut is_fill_main = false;
    let mut is_fixed_in_flex = false;

    if let Some(ref dim) = layout.width {
        match dim.dim_type {
            DimensionType::Fill => {
                if parent_is_row {
                    classes.push("flex-1".into());
                    classes.push("min-w-0".into());
                    is_fill_main = true;
                } else if parent_is_col {
                    classes.push("w-full".into());
                }
            }
            DimensionType::Hug => {}
            DimensionType::Fixed => {
                if let Some(val) = dim.value {
                    classes.push(dimension_class("w", val));
                    if in_flex {
                        is_fixed_in_flex = true;
                    }
                }
            }
        }
    }

    if let Some(ref dim) = layout.height {
        match dim.dim_type {
            DimensionType::Fill => {
                if parent_is_col {
                    // Main axis FILL in flex-col → grow
                    classes.push("flex-1".into());
                    classes.push("min-h-0".into());
                    is_fill_main = true;
                } else if parent_is_row {
                    // Cross axis FILL in flex-row → stretch to parent height
                    classes.push("h-full".into());
                }
            }
            DimensionType::Hug => {}
            DimensionType::Fixed => {
                if let Some(val) = dim.value {
                    classes.push(dimension_class("h", val));
                    if in_flex {
                        is_fixed_in_flex = true;
                    }
                }
            }
        }
    }

    // shrink-0: fixed-size elements in a flex parent should not shrink
    if is_fixed_in_flex && !is_fill_main {
        classes.push("shrink-0".into());
    }

    // Min/max size constraints
    if let Some(v) = layout.min_width {
        classes.push(dimension_class("min-w", v));
    }
    if let Some(v) = layout.max_width {
        classes.push(dimension_class("max-w", v));
    }
    if let Some(v) = layout.min_height {
        classes.push(dimension_class("min-h", v));
    }
    if let Some(v) = layout.max_height {
        classes.push(dimension_class("max-h", v));
    }

    // Self alignment
    if let Some(ref align) = layout.self_align {
        let cls = match align {
            Alignment::Start => "self-start",
            Alignment::Center => "self-center",
            Alignment::End => "self-end",
            Alignment::Stretch => "self-stretch",
            _ => "",
        };
        if !cls.is_empty() {
            classes.push(cls.into());
        }
    }

    classes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::schema::{Dimension, Padding};

    fn make_layout(mode: LayoutMode) -> Layout {
        Layout {
            mode: Some(mode),
            width: None,
            height: None,
            padding: None,
            gap: None,
            main_axis_align: None,
            cross_axis_align: None,
            constraints: None,
            position: None,
            overflow: None,
            rotation: None,
            parent_flex_dir: None,
            wrap: None,
            wrap_gap: None,
            wrap_align: None,
            min_width: None,
            max_width: None,
            min_height: None,
            max_height: None,
            self_align: None,
            overflow_x: None,
            overflow_y: None,
            z_index: None,
            aspect_ratio: None,
            grid_columns_sizing: None,
            grid_rows_sizing: None,
            grid_column_gap: None,
            grid_row_gap: None,
            grid_column_span: None,
            grid_row_span: None,
            grid_column_start: None,
            grid_row_start: None,
            flip_x: None,
            flip_y: None,
        }
    }

    #[test]
    fn test_horizontal_layout() {
        let layout = make_layout(LayoutMode::Horizontal);
        let classes = layout_classes(&layout);
        assert!(classes.contains(&"flex".to_string()));
        assert!(classes.contains(&"flex-row".to_string()));
    }

    #[test]
    fn test_vertical_layout() {
        let layout = make_layout(LayoutMode::Vertical);
        let classes = layout_classes(&layout);
        assert!(classes.contains(&"flex".to_string()));
        assert!(classes.contains(&"flex-col".to_string()));
    }

    #[test]
    fn test_gap() {
        let layout = Layout {
            gap: Some(8.0),
            ..make_layout(LayoutMode::Horizontal)
        };
        let classes = layout_classes(&layout);
        assert!(classes.contains(&"gap-[8px]".to_string()));
    }

    #[test]
    fn test_padding_uniform() {
        let layout = Layout {
            padding: Some(Padding {
                top: 16.0,
                right: 16.0,
                bottom: 16.0,
                left: 16.0,
            }),
            ..make_layout(LayoutMode::Vertical)
        };
        let classes = layout_classes(&layout);
        assert!(classes.contains(&"p-[16px]".to_string()));
    }

    #[test]
    fn test_padding_xy() {
        let layout = Layout {
            padding: Some(Padding {
                top: 8.0,
                right: 16.0,
                bottom: 8.0,
                left: 16.0,
            }),
            ..make_layout(LayoutMode::Vertical)
        };
        let classes = layout_classes(&layout);
        assert!(classes.contains(&"px-[16px]".to_string()));
        assert!(classes.contains(&"py-[8px]".to_string()));
    }

    #[test]
    fn test_padding_all_different() {
        let layout = Layout {
            padding: Some(Padding {
                top: 4.0,
                right: 8.0,
                bottom: 12.0,
                left: 16.0,
            }),
            ..make_layout(LayoutMode::Vertical)
        };
        let classes = layout_classes(&layout);
        assert!(classes.contains(&"pt-[4px]".to_string()));
        assert!(classes.contains(&"pr-[8px]".to_string()));
        assert!(classes.contains(&"pb-[12px]".to_string()));
        assert!(classes.contains(&"pl-[16px]".to_string()));
    }

    #[test]
    fn test_main_axis_align() {
        let layout = Layout {
            main_axis_align: Some(Alignment::Center),
            ..make_layout(LayoutMode::Horizontal)
        };
        let classes = layout_classes(&layout);
        assert!(classes.contains(&"justify-center".to_string()));
    }

    #[test]
    fn test_cross_axis_align() {
        let layout = Layout {
            cross_axis_align: Some(Alignment::Stretch),
            ..make_layout(LayoutMode::Horizontal)
        };
        let classes = layout_classes(&layout);
        assert!(classes.contains(&"items-stretch".to_string()));
    }

    #[test]
    fn test_width_fill_in_row() {
        let mut layout = make_layout(LayoutMode::None);
        layout.width = Some(Dimension {
            dim_type: DimensionType::Fill,
            value: None,
        });
        layout.parent_flex_dir = Some(LayoutMode::Horizontal);
        let classes = size_classes(&layout);
        assert!(classes.contains(&"flex-1".to_string()));
    }

    #[test]
    fn test_width_fill_in_col() {
        let mut layout = make_layout(LayoutMode::None);
        layout.width = Some(Dimension {
            dim_type: DimensionType::Fill,
            value: None,
        });
        layout.parent_flex_dir = Some(LayoutMode::Vertical);
        let classes = size_classes(&layout);
        // Cross-axis FILL in flex-col = default stretch, no class
        assert!(!classes.contains(&"flex-1".to_string()));
    }

    #[test]
    fn test_width_hug() {
        let layout = Layout {
            width: Some(Dimension {
                dim_type: DimensionType::Hug,
                value: None,
            }),
            ..make_layout(LayoutMode::None)
        };
        let classes = layout_classes(&layout);
        assert!(!classes.contains(&"w-fit".to_string()));
    }

    #[test]
    fn test_width_fixed() {
        let layout = Layout {
            width: Some(Dimension {
                dim_type: DimensionType::Fixed,
                value: Some(200.0),
            }),
            ..make_layout(LayoutMode::None)
        };
        let classes = layout_classes(&layout);
        assert!(classes.contains(&"w-[200px]".to_string()));
    }

    #[test]
    fn test_overflow() {
        let layout = Layout {
            overflow: Some(Overflow::Hidden),
            ..make_layout(LayoutMode::None)
        };
        let classes = layout_classes(&layout);
        assert!(classes.contains(&"overflow-hidden".to_string()));
    }

    #[test]
    fn test_overflow_scroll() {
        let layout = Layout {
            overflow: Some(Overflow::Scroll),
            ..make_layout(LayoutMode::None)
        };
        let classes = layout_classes(&layout);
        assert!(classes.contains(&"overflow-auto".to_string()));
    }

    #[test]
    fn test_space_between() {
        let layout = Layout {
            main_axis_align: Some(Alignment::SpaceBetween),
            ..make_layout(LayoutMode::Horizontal)
        };
        let classes = layout_classes(&layout);
        assert!(classes.contains(&"justify-between".to_string()));
    }
}
