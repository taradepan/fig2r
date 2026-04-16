use crate::emit::formatter::{sanitize_component_name, to_kebab_case};
use crate::ir::schema::Asset;
use base64::Engine;

pub fn svg_to_react_component(asset: &Asset) -> String {
    let component_name = format!("Icon{}", sanitize_component_name(&asset.name));
    let inner = extract_svg_inner(&asset.data);
    let viewbox = extract_attr(&asset.data, "viewBox").unwrap_or_else(|| "0 0 24 24".into());

    // Convert SVG inner content to JSX-safe: camelCase attrs + strip hardcoded fills
    let jsx_inner = svg_to_jsx(&strip_fill_attrs(&inner));

    format!(
        r#"interface {component_name}Props {{
  className?: string;
}}

export function {component_name}({{ className }}: {component_name}Props) {{
  return (
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="{viewbox}" className={{className}} fill="currentColor">
      {jsx_inner}
    </svg>
  );
}}
"#
    )
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
    svg.replace("fill-rule=", "fillRule=")
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
        .replace("pointer-events=", "pointerEvents=")
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
        assert!(output.contains("<path d=\"M5 13l4 4L19 7\""));
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
    fn test_svg_strips_hardcoded_fills() {
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
        // Hardcoded fill="#FF0000" should be stripped so it inherits currentColor
        assert!(!output.contains("#FF0000"));
        assert!(output.contains("fill=\"currentColor\"")); // outer svg still has it
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
}
