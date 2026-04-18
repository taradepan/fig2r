use crate::emit::formatter::{sanitize_component_name, to_kebab_case};
use crate::ir::schema::Asset;
use base64::Engine;

pub fn svg_to_react_component(asset: &Asset) -> String {
    svg_to_react_component_named(asset, None)
}

/// Same as [`svg_to_react_component`] but lets the caller pass an explicit
/// component name (e.g. after uniquification in `tree.rs`). When `None`, the
/// name is derived from `asset.name` via `sanitize_component_name`.
pub fn svg_to_react_component_named(asset: &Asset, override_name: Option<&str>) -> String {
    let component_name = override_name
        .map(str::to_string)
        .unwrap_or_else(|| format!("Icon{}", sanitize_component_name(&asset.name)));
    let inner = extract_svg_inner(&asset.data);
    let viewbox = extract_attr(&asset.data, "viewBox").unwrap_or_else(|| "0 0 24 24".into());
    let (default_w, default_h) = viewbox_to_dims(&viewbox);

    // Count explicit colors in the SVG. If there are none (everything uses
    // `currentColor`, `none`, or has no fill attr), the SVG is designed for CSS
    // theming — strip any lingering unthemed fills and set a `currentColor`
    // fallback on the outer `<svg>`. Otherwise preserve every fill as authored
    // so status dots, brand marks, and single-color decorative icons keep their
    // designed colors instead of collapsing to the parent text color (black).
    let colors = unique_fill_color_count(&asset.data);
    let themeable = colors == 0;
    let jsx_inner = if themeable {
        svg_to_jsx(&strip_fill_attrs(&inner))
    } else {
        svg_to_jsx(&inner)
    };
    // Preserve the outer `<svg fill="...">` Figma authored. Outline icons use
    // `fill="none"` so closed stroke-only paths don't default-fill to black in
    // the browser. For themeable icons with no authored fill, emit
    // `fill="currentColor"` so `className="text-red-500"` actually colors the
    // icon. Otherwise echo the authored attribute unchanged.
    let authored_fill = extract_attr(&asset.data, "fill");
    let outer_fill = if let Some(f) = authored_fill {
        format!(r#" fill="{f}""#)
    } else if themeable {
        r#" fill="currentColor""#.to_string()
    } else {
        String::new()
    };

    // Default width/height from viewBox give the SVG an intrinsic size so it
    // doesn't stretch to 100% of its parent flex container (browser default for
    // `<svg>` without explicit dimensions). Parent `className` can still override.
    format!(
        r#"interface {component_name}Props {{
  className?: string;
}}

export function {component_name}({{ className }}: {component_name}Props) {{
  return (
    <svg xmlns="http://www.w3.org/2000/svg" width="{default_w}" height="{default_h}" viewBox="{viewbox}" className={{className}}{outer_fill}>
      {jsx_inner}
    </svg>
  );
}}
"#
    )
}

/// Parse a viewBox string ("minX minY width height") and return (width, height)
/// as integer-formatted strings. Falls back to ("24", "24") for malformed input.
fn viewbox_to_dims(viewbox: &str) -> (String, String) {
    let parts: Vec<&str> = viewbox.split_whitespace().collect();
    if parts.len() != 4 {
        return ("24".into(), "24".into());
    }
    let parse = |s: &str| -> String {
        s.parse::<f64>()
            .ok()
            .map(|v| {
                if v.fract().abs() < 0.001 {
                    format!("{}", v.round() as i64)
                } else {
                    format!("{v}")
                }
            })
            .unwrap_or_else(|| "24".into())
    };
    (parse(parts[2]), parse(parts[3]))
}

pub fn optimize_svg(svg: &str) -> String {
    svg.lines().map(str::trim).collect::<String>()
}

pub fn svg_has_renderable_content(svg: &str) -> bool {
    let lower = svg.to_ascii_lowercase();
    [
        "<path",
        "<circle",
        "<rect",
        "<line",
        "<polyline",
        "<polygon",
        "<ellipse",
    ]
    .iter()
    .any(|tag| lower.contains(tag))
}

/// Count distinct authored colors in an SVG — across both `fill` and `stroke`
/// attributes. Ignores `none`, `currentColor`, and empty values. Outline icons
/// using `stroke="#..."` with no fills still count as having authored colors,
/// which prevents `strip_fill_attrs` + `fill="currentColor"` from collapsing
/// them into solid filled blobs. Colors within `<defs>` / `<clipPath>` / `<mask>`
/// blocks are skipped — those are invisible structural markup.
pub fn unique_fill_color_count(svg: &str) -> usize {
    let visible = strip_structural_blocks(svg);
    let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for attr in &["fill", "stroke"] {
        let needle = format!("{attr}=\"");
        let mut rest = visible.as_str();
        while let Some(pos) = rest.find(&needle) {
            let after = &rest[pos + needle.len()..];
            if let Some(end) = after.find('"') {
                let value = after[..end].trim();
                if !value.is_empty() && value != "none" && value != "currentColor" {
                    seen.insert(value.to_ascii_lowercase());
                }
                rest = &after[end + 1..];
            } else {
                break;
            }
        }
    }
    seen.len()
}

/// Remove content inside `<defs>`, `<clipPath>`, and `<mask>` blocks. Those
/// elements are rendered only when referenced (via `url(#...)`) and any fills
/// or strokes inside them don't contribute to the visible rendering — they
/// shouldn't poison the monochrome-color heuristic.
fn strip_structural_blocks(svg: &str) -> String {
    let mut out = String::with_capacity(svg.len());
    let mut rest = svg;
    loop {
        let Some(open_start) = ["<defs", "<clipPath", "<mask"]
            .iter()
            .filter_map(|tag| rest.find(tag).map(|p| (p, *tag)))
            .min_by_key(|&(p, _)| p)
        else {
            out.push_str(rest);
            break;
        };
        let (pos, tag) = open_start;
        out.push_str(&rest[..pos]);
        // Strip from `<tag` through matching `</tag...>`. Self-closing
        // `<tag ... />` (rare for these tags) also handled via the `/>` check.
        let after = &rest[pos..];
        let close_tag = format!("</{}>", &tag[1..]);
        if let Some(close_pos) = after.find(close_tag.as_str()) {
            rest = &after[close_pos + close_tag.len()..];
        } else if let Some(self_close) = after.find("/>") {
            rest = &after[self_close + 2..];
        } else {
            // Malformed — keep remainder as-is.
            out.push_str(after);
            break;
        }
    }
    out
}

/// Monochrome icons are suitable for the `fill="currentColor"` treatment
/// (className → color). Multi-color art is not — stripping fills turns it
/// into a solid silhouette. Use this to decide whether to generate a React
/// Icon component vs. fall back to `<img src="/assets/..svg">`.
pub fn is_monochrome_svg(svg: &str) -> bool {
    unique_fill_color_count(svg) <= 1
}

pub fn is_divider_svg(svg: &str) -> bool {
    let viewbox = extract_attr(svg, "viewBox").unwrap_or_default();
    let parts: Vec<_> = viewbox.split_whitespace().collect();
    if parts.len() != 4 {
        return false;
    }
    let Ok(w) = parts[2].parse::<f64>() else {
        return false;
    };
    let Ok(h) = parts[3].parse::<f64>() else {
        return false;
    };
    (w > 0.0 && h > 0.0) && (w / h > 10.0 || h / w > 10.0)
}

pub fn decode_image_asset(asset: &Asset) -> Result<Vec<u8>, base64::DecodeError> {
    base64::engine::general_purpose::STANDARD.decode(&asset.data)
}

pub fn asset_filename(name: &str, format: &str) -> String {
    let kebab = to_kebab_case(&sanitize_component_name(name));
    format!("{kebab}.{format}")
}

/// Strip hardcoded fill/stroke color attributes from SVG elements so they inherit from parent.
/// Preserves fill="none" and fill="currentColor".
fn strip_fill_attrs(svg: &str) -> String {
    let mut result = svg.to_string();
    for attr in &["fill", "stroke"] {
        let mut output = String::with_capacity(result.len());
        let mut rest = result.as_str();
        while let Some(pos) = rest.find(&format!("{attr}=\"")) {
            output.push_str(&rest[..pos]);
            let after_attr = &rest[pos + attr.len() + 2..]; // skip `attr="`
            if let Some(end) = after_attr.find('"') {
                let value = &after_attr[..end];
                if value == "none" || value == "currentColor" {
                    // Keep the attribute
                    output.push_str(&rest[pos..pos + attr.len() + 2 + end + 1]);
                } else {
                    // Strip the attribute — skip trailing space if present
                    // (output already excludes it)
                }
                rest = &after_attr[end + 1..];
                // Skip trailing space after stripped attribute
                if !value.is_empty()
                    && value != "none"
                    && value != "currentColor"
                    && rest.starts_with(' ')
                {
                    rest = &rest[1..];
                }
            } else {
                // Malformed — keep as-is
                output.push_str(&rest[pos..pos + attr.len() + 2]);
                rest = &rest[pos + attr.len() + 2..];
            }
        }
        output.push_str(rest);
        result = output;
    }
    result
}

/// Convert SVG attributes to JSX camelCase equivalents
fn svg_to_jsx(svg: &str) -> String {
    let camel = svg
        .replace("fill-rule=", "fillRule=")
        .replace("clip-rule=", "clipRule=")
        .replace("stroke-width=", "strokeWidth=")
        .replace("stroke-linecap=", "strokeLinecap=")
        .replace("stroke-linejoin=", "strokeLinejoin=")
        .replace("stroke-dasharray=", "strokeDasharray=")
        .replace("stroke-dashoffset=", "strokeDashoffset=")
        .replace("stroke-miterlimit=", "strokeMiterlimit=")
        .replace("stroke-opacity=", "strokeOpacity=")
        .replace("fill-opacity=", "fillOpacity=")
        .replace("clip-path=", "clipPath=")
        .replace("font-size=", "fontSize=")
        .replace("font-family=", "fontFamily=")
        .replace("font-weight=", "fontWeight=")
        .replace("text-anchor=", "textAnchor=")
        .replace("text-decoration=", "textDecoration=")
        .replace("dominant-baseline=", "dominantBaseline=")
        .replace("stop-color=", "stopColor=")
        .replace("stop-opacity=", "stopOpacity=")
        .replace("color-interpolation=", "colorInterpolation=")
        .replace("flood-color=", "floodColor=")
        .replace("flood-opacity=", "floodOpacity=")
        .replace("pointer-events=", "pointerEvents=");
    convert_inline_styles(&camel)
}

/// Convert `style="prop-one: v1; prop-two: v2"` to `style={{propOne: 'v1', propTwo: 'v2'}}`.
/// React rejects string `style`; it requires an object with camelCased keys.
fn convert_inline_styles(svg: &str) -> String {
    let mut result = String::with_capacity(svg.len());
    let mut rest = svg;
    while let Some(pos) = rest.find("style=\"") {
        result.push_str(&rest[..pos]);
        let after = &rest[pos + "style=\"".len()..];
        if let Some(end) = after.find('"') {
            let body = &after[..end];
            result.push_str("style={{");
            let mut first = true;
            for decl in body.split(';') {
                let decl = decl.trim();
                if decl.is_empty() {
                    continue;
                }
                let Some((k, v)) = decl.split_once(':') else {
                    continue;
                };
                let key = kebab_to_camel(k.trim());
                if key.is_empty() {
                    continue;
                }
                let val = v.trim().replace('\\', "\\\\").replace('\'', "\\'");
                if !first {
                    result.push_str(", ");
                }
                first = false;
                result.push_str(&key);
                result.push_str(": '");
                result.push_str(&val);
                result.push('\'');
            }
            result.push_str("}}");
            rest = &after[end + 1..];
        } else {
            // Malformed style attribute — preserve literal prefix, bail on search.
            result.push_str(&rest[pos..]);
            return result;
        }
    }
    result.push_str(rest);
    result
}

fn kebab_to_camel(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut cap = false;
    for ch in s.chars() {
        if ch == '-' {
            cap = true;
        } else if cap {
            out.extend(ch.to_uppercase());
            cap = false;
        } else {
            out.push(ch);
        }
    }
    out
}

fn extract_svg_inner(svg: &str) -> String {
    // Find the <svg opening tag, then its closing >, to skip XML declarations
    if let Some(svg_start) = svg.find("<svg")
        && let Some(tag_end) = svg[svg_start..].find('>')
        && let Some(end) = svg.rfind("</svg>")
    {
        let start = svg_start + tag_end;
        return svg[start + 1..end].trim().to_string();
    }
    String::new()
}

fn extract_attr(svg: &str, attr: &str) -> Option<String> {
    let pattern = format!("{attr}=\"");
    if let Some(start) = svg.find(&pattern) {
        let rest = &svg[start + pattern.len()..];
        if let Some(end) = rest.find('"') {
            return Some(rest[..end].to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::schema::AssetType;

    #[test]
    fn test_svg_to_react_component() {
        let asset = Asset {
            id: "icon-1".into(),
            name: "check".into(),
            asset_type: AssetType::Svg,
            format: "svg".into(),
            data: r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M5 13l4 4L19 7"/></svg>"#.into(),
            url: None,
            source_ref: None,
        };
        let output = svg_to_react_component(&asset);
        assert!(output.contains("export function IconCheck"));
        assert!(output.contains("viewBox=\"0 0 24 24\""));
        assert!(output.contains("width=\"24\""));
        assert!(output.contains("height=\"24\""));
        assert!(output.contains("<path d=\"M5 13l4 4L19 7\""));
    }

    #[test]
    fn test_svg_intrinsic_dims_from_viewbox() {
        let asset = Asset {
            id: "i".into(),
            name: "tiny".into(),
            asset_type: AssetType::Svg,
            format: "svg".into(),
            data: r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><path d="M0 0"/></svg>"#.into(),
            url: None,
            source_ref: None,
        };
        let output = svg_to_react_component(&asset);
        assert!(output.contains("width=\"16\""));
        assert!(output.contains("height=\"16\""));
    }

    #[test]
    fn test_svg_missing_viewbox_defaults() {
        let asset = Asset {
            id: "i".into(),
            name: "bare".into(),
            asset_type: AssetType::Svg,
            format: "svg".into(),
            data: r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="M0 0"/></svg>"#.into(),
            url: None,
            source_ref: None,
        };
        let output = svg_to_react_component(&asset);
        assert!(output.contains("width=\"24\""));
        assert!(output.contains("height=\"24\""));
    }

    #[test]
    fn test_svg_component_naming() {
        let asset = Asset {
            id: "a".into(),
            name: "arrow-left".into(),
            asset_type: AssetType::Svg,
            format: "svg".into(),
            data: "<svg></svg>".into(),
            url: None,
            source_ref: None,
        };
        let output = svg_to_react_component(&asset);
        assert!(output.contains("export function IconArrowLeft"));
    }

    #[test]
    fn test_svg_preserves_authored_fills_for_colored_icons() {
        let asset = Asset {
            id: "icon-2".into(),
            name: "star".into(),
            asset_type: AssetType::Svg,
            format: "svg".into(),
            data: r##"<svg viewBox="0 0 24 24"><path fill="#FF0000" d="M12 2L15 9H21L16 14L18 21L12 17L6 21L8 14L3 9H9Z"/></svg>"##.into(),
            url: None,
            source_ref: None,
        };
        let output = svg_to_react_component(&asset);
        // SVG has an explicit color — preserve it (don't theme) so status dots
        // and brand marks keep their designed color.
        assert!(output.contains("#FF0000"));
        assert!(!output.contains("fill=\"currentColor\""));
    }

    #[test]
    fn test_svg_themes_when_no_explicit_colors() {
        let asset = Asset {
            id: "icon-themeable".into(),
            name: "check".into(),
            asset_type: AssetType::Svg,
            format: "svg".into(),
            data: r#"<svg viewBox="0 0 24 24"><path d="M5 13l4 4L19 7"/></svg>"#.into(),
            url: None,
            source_ref: None,
        };
        let output = svg_to_react_component(&asset);
        // No explicit colors — fall back to currentColor so className can theme it.
        assert!(output.contains("fill=\"currentColor\""));
    }

    #[test]
    fn test_svg_preserves_fill_none() {
        let asset = Asset {
            id: "icon-3".into(),
            name: "outline".into(),
            asset_type: AssetType::Svg,
            format: "svg".into(),
            data: r#"<svg viewBox="0 0 24 24"><path fill="none" stroke-width="2" d="M5 13l4 4L19 7"/></svg>"#.into(),
            url: None,
            source_ref: None,
        };
        let output = svg_to_react_component(&asset);
        assert!(output.contains("fill=\"none\""));
    }

    #[test]
    fn test_svg_camelcase_attrs() {
        let asset = Asset {
            id: "icon-4".into(),
            name: "line".into(),
            asset_type: AssetType::Svg,
            format: "svg".into(),
            data: r#"<svg viewBox="0 0 24 24"><path fill-rule="evenodd" clip-rule="nonzero" stroke-width="2" stroke-linecap="round" d="M0 0"/></svg>"#.into(),
            url: None,
            source_ref: None,
        };
        let output = svg_to_react_component(&asset);
        assert!(output.contains("fillRule="));
        assert!(output.contains("clipRule="));
        assert!(output.contains("strokeWidth="));
        assert!(output.contains("strokeLinecap="));
        assert!(!output.contains("fill-rule="));
        assert!(!output.contains("clip-rule="));
        assert!(!output.contains("stroke-width="));
    }

    #[test]
    fn test_image_asset_decode() {
        let data = base64::engine::general_purpose::STANDARD.encode(b"fake-png-data");
        let asset = Asset {
            id: "img-1".into(),
            name: "hero".into(),
            asset_type: AssetType::Image,
            format: "png".into(),
            data,
            url: None,
            source_ref: None,
        };
        let bytes = decode_image_asset(&asset).unwrap();
        assert_eq!(bytes, b"fake-png-data");
    }

    #[test]
    fn test_asset_filename() {
        assert_eq!(asset_filename("hero image", "png"), "hero-image.png");
        assert_eq!(asset_filename("Icon/Check", "svg"), "icon-check.svg");
    }

    #[test]
    fn test_svg_has_renderable_content() {
        assert!(svg_has_renderable_content("<svg><path d=\"M0 0\"/></svg>"));
        assert!(!svg_has_renderable_content("<svg></svg>"));
    }

    #[test]
    fn test_is_divider_svg() {
        assert!(is_divider_svg("<svg viewBox=\"0 0 200 1\"></svg>"));
        assert!(!is_divider_svg("<svg viewBox=\"0 0 24 24\"></svg>"));
    }

    #[test]
    fn test_monochrome_svg_classification() {
        assert!(is_monochrome_svg(
            r##"<svg><path fill="#000" d="M0 0"/><path fill="#000" d="M1 1"/></svg>"##
        ));
        assert!(is_monochrome_svg(
            r#"<svg><path fill="none" d="M0 0"/><circle fill="currentColor"/></svg>"#
        ));
        assert!(!is_monochrome_svg(
            r##"<svg><path fill="#FF0000"/><path fill="#00FF00"/><path fill="#0000FF"/></svg>"##
        ));
        assert!(is_monochrome_svg(
            r##"<svg><path fill="#FFFFFF"/><path fill="#ffffff"/></svg>"##
        ));
        assert!(is_monochrome_svg(r#"<svg><path d="M0 0"/></svg>"#));
    }

    #[test]
    fn test_outer_fill_none_preserved_for_outline_icons() {
        // Figma exports outline icons with outer `fill="none"` so closed stroke
        // paths don't default-fill to black. We must preserve that attribute.
        let asset = Asset {
            id: "i".into(),
            name: "outline".into(),
            asset_type: AssetType::Svg,
            format: "svg".into(),
            data: r##"<svg viewBox="0 0 14 14" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M7 0C10.866 0 14 3.134 14 7C14 10.866 10.866 14 7 14Z" stroke="#7B7B7B"/></svg>"##.into(),
            url: None,
            source_ref: None,
        };
        let output = svg_to_react_component(&asset);
        assert!(
            output.contains("fill=\"none\""),
            "outer fill=\"none\" must be preserved, got:\n{output}"
        );
        assert!(!output.contains("fill=\"currentColor\""));
    }

    #[test]
    fn test_structural_block_colors_ignored() {
        // fill="white" inside <defs><clipPath><rect/></clipPath></defs> is a mask
        // shape, not a visible authored color — don't let it flip a monochrome icon.
        let svg = r##"<svg viewBox="0 0 14 14"><g clip-path="url(#c)"><path stroke="#7B7B7B" d="M0 0"/></g><defs><clipPath id="c"><rect fill="white"/></clipPath></defs></svg>"##;
        assert_eq!(unique_fill_color_count(svg), 1);
        assert!(is_monochrome_svg(svg));
    }

    #[test]
    fn test_stroke_colors_also_count() {
        // Stroke-based outline icons must be treated as having authored colors
        // so we don't strip their strokes and leave a blank filled shape.
        assert_eq!(
            unique_fill_color_count(
                r##"<svg><path stroke="#7B7B7B" d="M0 0"/><path stroke="#7B7B7B" d="M1 1"/></svg>"##
            ),
            1
        );
        assert_eq!(
            unique_fill_color_count(r##"<svg><path fill="#FF0000" stroke="#00FF00"/></svg>"##),
            2
        );
        // stroke="none" and stroke="currentColor" don't count.
        assert_eq!(
            unique_fill_color_count(
                r#"<svg><path stroke="none"/><path stroke="currentColor"/></svg>"#
            ),
            0
        );
    }

    #[test]
    fn test_svg_inline_style_becomes_jsx_object() {
        let asset = Asset {
            id: "m".into(),
            name: "masky".into(),
            asset_type: AssetType::Svg,
            format: "svg".into(),
            data: r#"<svg viewBox="0 0 24 24"><mask id="m" style="mask-type:luminance"/></svg>"#
                .into(),
            url: None,
            source_ref: None,
        };
        let output = svg_to_react_component(&asset);
        assert!(
            !output.contains("style=\"mask-type:luminance\""),
            "raw HTML-style attribute must be rewritten"
        );
        assert!(output.contains("style={{maskType: 'luminance'}}"));
    }

    #[test]
    fn test_svg_multiple_style_declarations() {
        let asset = Asset {
            id: "m".into(),
            name: "multi".into(),
            asset_type: AssetType::Svg,
            format: "svg".into(),
            data: r#"<svg viewBox="0 0 24 24"><rect style="mix-blend-mode: multiply; mask-type: alpha"/></svg>"#
                .into(),
            url: None,
            source_ref: None,
        };
        let output = svg_to_react_component(&asset);
        assert!(output.contains("style={{mixBlendMode: 'multiply', maskType: 'alpha'}}"));
    }

    #[test]
    fn test_svg_empty_style_is_dropped() {
        let asset = Asset {
            id: "m".into(),
            name: "empty".into(),
            asset_type: AssetType::Svg,
            format: "svg".into(),
            data: r#"<svg viewBox="0 0 24 24"><rect style=""/></svg>"#.into(),
            url: None,
            source_ref: None,
        };
        let output = svg_to_react_component(&asset);
        assert!(output.contains("style={{}}"));
    }
}
