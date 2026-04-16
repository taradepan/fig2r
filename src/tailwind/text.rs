use crate::ir::schema::{TextAlign, TextDecoration, TextProps, TextTransform, Truncation};
use crate::tailwind::values;

/// CSS variable name emitted for a google font family (e.g. `--font-jetbrains-mono`).
/// Must match the `variable:` option in the component's next/font import.
pub fn google_font_css_var(family: &str) -> String {
    let mut out = String::new();
    for ch in family.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('-');
        }
    }
    while out.contains("--") {
        out = out.replace("--", "-");
    }
    format!("--font-{}", out.trim_matches('-'))
}

pub fn text_classes(text: &TextProps) -> Vec<String> {
    let mut classes = Vec::new();

    // Font family — emit Tailwind arbitrary class pointing at the CSS variable
    // populated by next/font (see `google_font_css_var` in codegen).
    // Inter is treated as the default body font and omitted.
    if let Some(ref family) = text.font_family
        && family != "Inter"
    {
        let css_var = google_font_css_var(family);
        classes.push(format!("font-[var({css_var})]"));
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
        match decoration {
            TextDecoration::Underline => classes.push("underline".into()),
            TextDecoration::Strikethrough => classes.push("line-through".into()),
            TextDecoration::None => {}
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
