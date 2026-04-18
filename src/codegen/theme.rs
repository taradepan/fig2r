use crate::ir::schema::Theme;
use std::collections::HashMap;

pub fn generate_tailwind_extend(theme: &Theme) -> String {
    let mut sections = Vec::new();

    if let Some(ref colors) = theme.colors {
        sections.push(format!("  colors: {}", format_string_map(colors)));
    }
    if let Some(ref spacing) = theme.spacing {
        sections.push(format!("  spacing: {}", format_string_map(spacing)));
    }
    if let Some(ref br) = theme.border_radius {
        sections.push(format!("  borderRadius: {}", format_string_map(br)));
    }
    if let Some(ref fs) = theme.font_size {
        sections.push(format!("  fontSize: {}", format_string_map(fs)));
    }
    if let Some(ref ff) = theme.font_family {
        sections.push(format!("  fontFamily: {}", format_font_family_map(ff)));
    }
    if let Some(ref shadows) = theme.shadows {
        sections.push(format!("  boxShadow: {}", format_string_map(shadows)));
    }
    if let Some(ref opacity) = theme.opacity {
        sections.push(format!("  opacity: {}", format_number_map(opacity)));
    }

    let body = sections.join(",\n");
    format!("module.exports = {{\n{body}\n}};\n")
}

pub fn generate_tokens_ts(theme: &Theme) -> String {
    let mut parts = Vec::new();

    if let Some(ref colors) = theme.colors {
        parts.push(format!(
            "export const colors = {} as const;",
            format_ts_string_map(colors)
        ));
    }
    if let Some(ref spacing) = theme.spacing {
        parts.push(format!(
            "export const spacing = {} as const;",
            format_ts_string_map(spacing)
        ));
    }
    if let Some(ref br) = theme.border_radius {
        parts.push(format!(
            "export const borderRadius = {} as const;",
            format_ts_string_map(br)
        ));
    }
    if let Some(ref fs) = theme.font_size {
        parts.push(format!(
            "export const fontSize = {} as const;",
            format_ts_string_map(fs)
        ));
    }
    if let Some(ref ff) = theme.font_family {
        parts.push(format!(
            "export const fontFamily = {} as const;",
            format_ts_string_map(ff)
        ));
    }
    if let Some(ref shadows) = theme.shadows {
        parts.push(format!(
            "export const shadows = {} as const;",
            format_ts_string_map(shadows)
        ));
    }
    if let Some(ref opacity) = theme.opacity {
        parts.push(format!(
            "export const opacity = {} as const;",
            format_ts_number_map(opacity)
        ));
    }

    parts.join("\n\n") + "\n"
}

fn format_string_map(map: &HashMap<String, String>) -> String {
    let mut entries: Vec<_> = map.iter().collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    let items: Vec<String> = entries
        .iter()
        .map(|(k, v)| {
            if k.contains('-') || k.contains(' ') {
                format!("    '{k}': '{v}'")
            } else {
                format!("    {k}: '{v}'")
            }
        })
        .collect();
    format!("{{\n{}\n  }}", items.join(",\n"))
}

fn format_font_family_map(map: &HashMap<String, String>) -> String {
    let mut entries: Vec<_> = map.iter().collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    let items: Vec<String> = entries
        .iter()
        .map(|(k, v)| format!("    {k}: ['{v}', 'sans-serif']"))
        .collect();
    format!("{{\n{}\n  }}", items.join(",\n"))
}

fn format_number_map(map: &HashMap<String, f64>) -> String {
    let mut entries: Vec<_> = map.iter().collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    let items: Vec<String> = entries
        .iter()
        .map(|(k, v)| format!("    {k}: {v}"))
        .collect();
    format!("{{\n{}\n  }}", items.join(",\n"))
}

fn format_ts_string_map(map: &HashMap<String, String>) -> String {
    let mut entries: Vec<_> = map.iter().collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    let items: Vec<String> = entries
        .iter()
        .map(|(k, v)| {
            if k.contains('-') || k.contains(' ') {
                format!("  '{k}': '{v}'")
            } else {
                format!("  {k}: '{v}'")
            }
        })
        .collect();
    format!("{{\n{}\n}}", items.join(",\n"))
}

fn format_ts_number_map(map: &HashMap<String, f64>) -> String {
    let mut entries: Vec<_> = map.iter().collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    let items: Vec<String> = entries.iter().map(|(k, v)| format!("  {k}: {v}")).collect();
    format!("{{\n{}\n}}", items.join(",\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::schema::*;
    use std::collections::HashMap;

    fn make_theme() -> Theme {
        Theme {
            colors: Some(HashMap::from([
                ("primary".into(), "#3B82F6".into()),
                ("secondary".into(), "#F3F4F6".into()),
            ])),
            spacing: Some(HashMap::from([("sm".into(), "4px".into())])),
            border_radius: Some(HashMap::from([("md".into(), "8px".into())])),
            font_size: Some(HashMap::from([("base".into(), "16px".into())])),
            font_family: Some(HashMap::from([("sans".into(), "Inter".into())])),
            shadows: Some(HashMap::from([(
                "sm".into(),
                "0 1px 2px rgba(0,0,0,0.05)".into(),
            )])),
            opacity: Some(HashMap::from([("disabled".into(), 0.5)])),
        }
    }

    #[test]
    fn test_generate_tailwind_extend() {
        let theme = make_theme();
        let output = generate_tailwind_extend(&theme);
        assert!(output.contains("module.exports"));
        assert!(output.contains("primary"));
        assert!(output.contains("#3B82F6"));
        assert!(output.contains("Inter"));
    }

    #[test]
    fn test_generate_tokens_ts() {
        let theme = make_theme();
        let output = generate_tokens_ts(&theme);
        assert!(output.contains("export const colors"));
        assert!(output.contains("primary"));
        assert!(output.contains("#3B82F6"));
    }

    #[test]
    fn test_empty_theme() {
        let theme = Theme {
            colors: None,
            spacing: None,
            border_radius: None,
            font_size: None,
            font_family: None,
            shadows: None,
            opacity: None,
        };
        let output = generate_tailwind_extend(&theme);
        assert!(output.contains("module.exports"));
    }
}
