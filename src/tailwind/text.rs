use crate::codegen::fonts::{custom_font_fallback_class, is_google_font};
use crate::ir::schema::{
    TextAlign, TextDecoration, TextDecorationStyle, TextProps, TextTransform, Truncation,
};
use crate::tailwind::values;

/// CSS variable name emitted for a google font family (e.g. `--font-jetbrains-mono`).
/// Must match the `variable:` option in the component's next/font import.
pub fn google_font_css_var(family: &str) -> String {
    let mut out = String::with_capacity(family.len() + 7);
    out.push_str("--font-");
    let mut last_was_sep = true;
    for ch in family.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_was_sep = false;
        } else if !last_was_sep {
            out.push('-');
            last_was_sep = true;
        }
    }
    if out.ends_with('-') {
        out.pop();
    }
    out
}

pub fn text_classes(text: &TextProps) -> Vec<String> {
    let mut classes = Vec::new();

    // Font family.
    // - "Inter" is the default body font → omit the class entirely.
    // - Other Google fonts → emit `font-[var(--font-X)]`; the CSS variable is
    //   populated by the `next/font/google` import in the generated component.
    // - Custom (non-Google) fonts → emit a generic `font-serif` / `font-sans`
    //   fallback. Emitting the CSS variable would be a dead reference (the
    //   component has no @font-face for it), so the text would fall back to
    //   the browser default. A user-visible warning is raised separately in
    //   `codegen::component::generate_font_block`.
    if let Some(ref family) = text.font_family
        && family != "Inter"
    {
        if is_google_font(family) {
            let css_var = google_font_css_var(family);
            classes.push(format!("font-[var({css_var})]"));
        } else {
            classes.push(custom_font_fallback_class(family).to_string());
        }
    }

    if let Some(size) = text.font_size {
        classes.push(values::font_size_class(size));
    }

    if let Some(weight) = text.font_weight {
        classes.push(values::font_weight_class(weight));
    }

    if let Some(ref align) = text.text_align {
        classes.push(match align {
            TextAlign::Left => "text-left".into(),
            TextAlign::Center => "text-center".into(),
            TextAlign::Right => "text-right".into(),
            TextAlign::Justify => "text-justify".into(),
        });
    }

    if let Some(ref decoration) = text.text_decoration {
        let is_decorated = match decoration {
            TextDecoration::Underline => {
                classes.push("underline".into());
                true
            }
            TextDecoration::Strikethrough => {
                classes.push("line-through".into());
                true
            }
            TextDecoration::None => false,
        };

        if is_decorated {
            // Decoration style — Solid is the default, skip it to keep output clean.
            if let Some(ref style) = text.text_decoration_style {
                match style {
                    TextDecorationStyle::Solid => {}
                    TextDecorationStyle::Double => classes.push("decoration-double".into()),
                    TextDecorationStyle::Dotted => classes.push("decoration-dotted".into()),
                    TextDecorationStyle::Dashed => classes.push("decoration-dashed".into()),
                    TextDecorationStyle::Wavy => classes.push("decoration-wavy".into()),
                }
            }

            // Underline offset (distance from baseline to decoration line).
            if let Some(offset) = text.text_decoration_offset {
                classes.push(format!("underline-offset-[{offset}px]"));
            }

            // Decoration line thickness.
            if let Some(thickness) = text.text_decoration_thickness {
                classes.push(format!("decoration-[{thickness}px]"));
            }
        }
    }

    if let Some(ref transform) = text.text_transform {
        match transform {
            TextTransform::Uppercase => classes.push("uppercase".into()),
            TextTransform::Lowercase => classes.push("lowercase".into()),
            TextTransform::Capitalize => classes.push("capitalize".into()),
            TextTransform::SmallCaps => classes.push("[font-variant-caps:small-caps]".into()),
            TextTransform::AllSmallCaps => {
                classes.push("[font-variant-caps:all-small-caps]".into());
            }
            TextTransform::None => {}
        }
    }

    if text.italic == Some(true) {
        classes.push("italic".into());
    }

    // OpenType feature flags → font-feature-settings
    if let Some(ref flags) = text.opentype_flags
        && !flags.is_empty()
    {
        let mut features: Vec<String> = flags
            .iter()
            .map(|(tag, val)| format!("'{tag}'_{val}"))
            .collect();
        features.sort();
        classes.push(format!("[font-feature-settings:{}]", features.join(",")));
    }

    // Handle truncation and max_lines together — they conflict:
    // - truncate: single-line, overflow hidden, ellipsis, white-space: nowrap
    // - line-clamp-N: multi-line, -webkit-line-clamp
    // Priority: max_lines > 1 wins (multi-line), otherwise truncate for single-line ellipsis
    if let Some(max_lines) = text.max_lines
        && max_lines > 0
    {
        if max_lines == 1 {
            classes.push("truncate".into());
        } else {
            classes.push(format!("line-clamp-{max_lines}"));
        }
    } else if let Some(ref truncation) = text.truncation
        && *truncation == Truncation::Ellipsis
    {
        classes.push("truncate".into());
    }

    if let Some(lh) = text.line_height {
        classes.push(line_height_class(lh));
    }

    if let Some(ls) = text.letter_spacing
        && ls.abs() > 0.001
    {
        classes.push(format!("tracking-[{ls}em]"));
    }

    classes
}

fn line_height_class(value: f64) -> String {
    let scale: &[(f64, &str)] = &[
        (1.0, "none"),
        (1.25, "tight"),
        (1.375, "snug"),
        (1.5, "normal"),
        (1.625, "relaxed"),
        (2.0, "loose"),
    ];
    for &(val, name) in scale {
        if (value - val).abs() < 0.01 {
            return format!("leading-{name}");
        }
    }
    format!("leading-[{value}]")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_text(content: &str) -> TextProps {
        TextProps {
            content: content.into(),
            font_size: None,
            font_family: None,
            font_weight: None,
            line_height: None,
            letter_spacing: None,
            text_align: None,
            text_decoration: None,
            text_decoration_style: None,
            text_decoration_offset: None,
            text_decoration_thickness: None,
            text_transform: None,
            truncation: None,
            italic: None,
            vertical_align: None,
            paragraph_spacing: None,
            max_lines: None,
            hyperlink: None,
            list_type: None,
            opentype_flags: None,
            spans: None,
        }
    }

    #[test]
    fn test_basic_text() {
        let text = make_text("Hello");
        let classes = text_classes(&text);
        assert!(classes.is_empty());
    }

    #[test]
    fn test_font_size() {
        let text = TextProps {
            font_size: Some(16.0),
            ..make_text("Hi")
        };
        let classes = text_classes(&text);
        assert!(classes.contains(&"text-[16px]".to_string()));
    }

    #[test]
    fn test_font_weight() {
        let text = TextProps {
            font_weight: Some(700),
            ..make_text("Bold")
        };
        let classes = text_classes(&text);
        assert!(classes.contains(&"font-bold".to_string()));
    }

    #[test]
    fn test_text_align() {
        let text = TextProps {
            text_align: Some(TextAlign::Center),
            ..make_text("Centered")
        };
        let classes = text_classes(&text);
        assert!(classes.contains(&"text-center".to_string()));
    }

    #[test]
    fn test_text_decoration_underline() {
        let text = TextProps {
            text_decoration: Some(TextDecoration::Underline),
            ..make_text("Link")
        };
        let classes = text_classes(&text);
        assert!(classes.contains(&"underline".to_string()));
    }

    #[test]
    fn test_text_decoration_strikethrough() {
        let text = TextProps {
            text_decoration: Some(TextDecoration::Strikethrough),
            ..make_text("Old")
        };
        let classes = text_classes(&text);
        assert!(classes.contains(&"line-through".to_string()));
    }

    #[test]
    fn test_decoration_wavy_emits_decoration_wavy_class() {
        let text = TextProps {
            text_decoration: Some(TextDecoration::Underline),
            text_decoration_style: Some(TextDecorationStyle::Wavy),
            ..make_text("Spelling")
        };
        let classes = text_classes(&text);
        assert!(classes.contains(&"underline".to_string()));
        assert!(classes.contains(&"decoration-wavy".to_string()));
    }

    #[test]
    fn test_decoration_style_solid_is_default_and_omitted() {
        let text = TextProps {
            text_decoration: Some(TextDecoration::Underline),
            text_decoration_style: Some(TextDecorationStyle::Solid),
            ..make_text("Link")
        };
        let classes = text_classes(&text);
        assert!(classes.contains(&"underline".to_string()));
        assert!(!classes.iter().any(|c| c.starts_with("decoration-")));
    }

    #[test]
    fn test_decoration_offset_emits_underline_offset() {
        let text = TextProps {
            text_decoration: Some(TextDecoration::Underline),
            text_decoration_offset: Some(4.0),
            ..make_text("Link")
        };
        let classes = text_classes(&text);
        assert!(classes.contains(&"underline-offset-[4px]".to_string()));
    }

    #[test]
    fn test_decoration_thickness_emits_arbitrary_class() {
        let text = TextProps {
            text_decoration: Some(TextDecoration::Underline),
            text_decoration_thickness: Some(2.0),
            ..make_text("Link")
        };
        let classes = text_classes(&text);
        assert!(classes.contains(&"decoration-[2px]".to_string()));
    }

    #[test]
    fn test_no_decoration_classes_without_decoration() {
        // Offset/thickness/style must not leak when text_decoration is None.
        let text = TextProps {
            text_decoration: None,
            text_decoration_style: Some(TextDecorationStyle::Wavy),
            text_decoration_offset: Some(4.0),
            text_decoration_thickness: Some(2.0),
            ..make_text("Plain")
        };
        let classes = text_classes(&text);
        assert!(
            !classes
                .iter()
                .any(|c| c.starts_with("decoration-") || c.starts_with("underline-offset-"))
        );
    }

    #[test]
    fn test_decoration_none_variant_suppresses_subfields() {
        // TextDecoration::None should be treated as no decoration.
        let text = TextProps {
            text_decoration: Some(TextDecoration::None),
            text_decoration_style: Some(TextDecorationStyle::Wavy),
            text_decoration_offset: Some(4.0),
            text_decoration_thickness: Some(2.0),
            ..make_text("Plain")
        };
        let classes = text_classes(&text);
        assert!(
            !classes
                .iter()
                .any(|c| c.starts_with("decoration-") || c.starts_with("underline-offset-"))
        );
    }

    #[test]
    fn test_text_transform_uppercase() {
        let text = TextProps {
            text_transform: Some(TextTransform::Uppercase),
            ..make_text("LOUD")
        };
        let classes = text_classes(&text);
        assert!(classes.contains(&"uppercase".to_string()));
    }

    #[test]
    fn test_truncation_ellipsis() {
        let text = TextProps {
            truncation: Some(Truncation::Ellipsis),
            ..make_text("Long...")
        };
        let classes = text_classes(&text);
        assert!(classes.contains(&"truncate".to_string()));
    }

    #[test]
    fn test_line_height_known() {
        let text = TextProps {
            line_height: Some(1.5),
            ..make_text("Body")
        };
        let classes = text_classes(&text);
        assert!(classes.contains(&"leading-normal".to_string()));
    }

    #[test]
    fn test_line_height_arbitrary() {
        let text = TextProps {
            line_height: Some(1.8),
            ..make_text("Custom")
        };
        let classes = text_classes(&text);
        assert!(classes.contains(&"leading-[1.8]".to_string()));
    }

    #[test]
    fn test_letter_spacing() {
        let text = TextProps {
            letter_spacing: Some(0.05),
            ..make_text("Spaced")
        };
        let classes = text_classes(&text);
        assert!(classes.contains(&"tracking-[0.05em]".to_string()));
    }

    #[test]
    fn test_google_font_emits_css_var() {
        let text = TextProps {
            font_family: Some("JetBrains Mono".into()),
            ..make_text("code")
        };
        let classes = text_classes(&text);
        assert!(classes.contains(&"font-[var(--font-jetbrains-mono)]".to_string()));
    }

    #[test]
    fn test_inter_font_is_omitted() {
        let text = TextProps {
            font_family: Some("Inter".into()),
            ..make_text("body")
        };
        let classes = text_classes(&text);
        assert!(!classes.iter().any(|c| c.starts_with("font-")));
    }

    #[test]
    fn test_custom_font_falls_back_to_font_serif() {
        let text = TextProps {
            font_family: Some("Perfectly Nineties".into()),
            ..make_text("headline")
        };
        let classes = text_classes(&text);
        // Heuristic: no serif/roman/times hint → font-sans.
        assert!(classes.contains(&"font-sans".to_string()));
        assert!(
            !classes
                .iter()
                .any(|c| c.contains("var(--font-perfectly-nineties)"))
        );
    }

    #[test]
    fn test_custom_serif_font_falls_back_to_font_serif() {
        let text = TextProps {
            font_family: Some("Perfectly Nineties Serif".into()),
            ..make_text("headline")
        };
        let classes = text_classes(&text);
        assert!(classes.contains(&"font-serif".to_string()));
        assert!(!classes.iter().any(|c| c.contains("var(--font-")));
    }

    #[test]
    fn test_combined_text_styles() {
        let text = TextProps {
            font_size: Some(24.0),
            font_weight: Some(600),
            text_align: Some(TextAlign::Center),
            ..make_text("Heading")
        };
        let classes = text_classes(&text);
        assert!(classes.contains(&"text-[24px]".to_string()));
        assert!(classes.contains(&"font-semibold".to_string()));
        assert!(classes.contains(&"text-center".to_string()));
    }
}
