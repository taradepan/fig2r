use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct VariantProp {
    pub name: String,
    pub values: Vec<String>,
    pub default: String,
}

pub fn extract_variant_props(
    variants: &HashMap<String, Vec<String>>,
    defaults: &HashMap<String, String>,
) -> Vec<VariantProp> {
    let mut props: Vec<VariantProp> = variants
        .iter()
        .map(|(name, values)| VariantProp {
            name: name.clone(),
            values: values.clone(),
            default: defaults
                .get(name)
                .cloned()
                .unwrap_or_else(|| values[0].clone()),
        })
        .collect();
    props.sort_by(|a, b| a.name.cmp(&b.name));
    props
}

pub fn generate_prop_interface(
    component_name: &str,
    props: &[VariantProp],
    has_children: bool,
) -> String {
    let mut lines = vec![format!("interface {component_name}Props {{")];
    for prop in props {
        let union: String = prop
            .values
            .iter()
            .map(|v| format!("'{v}'"))
            .collect::<Vec<_>>()
            .join(" | ");
        lines.push(format!("  {}?: {};", prop.name, union));
    }
    if has_children {
        lines.push("  children?: React.ReactNode;".into());
    }
    lines.push("}".into());
    lines.join("\n")
}

pub fn generate_destructure(props: &[VariantProp], has_children: bool) -> String {
    let mut parts: Vec<String> = props
        .iter()
        .map(|p| format!("{} = '{}'", p.name, p.default))
        .collect();
    if has_children {
        parts.push("children".into());
    }
    format!("{{ {} }}", parts.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_extract_variant_props() {
        let variants = HashMap::from([
            ("size".into(), vec!["sm".into(), "md".into(), "lg".into()]),
            ("variant".into(), vec!["primary".into(), "secondary".into()]),
        ]);
        let defaults = HashMap::from([
            ("size".into(), "md".into()),
            ("variant".into(), "primary".into()),
        ]);
        let props = extract_variant_props(&variants, &defaults);
        assert_eq!(props.len(), 2);
        let size_prop = props.iter().find(|p| p.name == "size").unwrap();
        assert_eq!(size_prop.default, "md");
        assert_eq!(size_prop.values, vec!["sm", "md", "lg"]);
    }

    #[test]
    fn test_generate_prop_interface() {
        let props = vec![
            VariantProp {
                name: "size".into(),
                values: vec!["sm".into(), "md".into()],
                default: "md".into(),
            },
            VariantProp {
                name: "variant".into(),
                values: vec!["primary".into(), "secondary".into()],
                default: "primary".into(),
            },
        ];
        let output = generate_prop_interface("Button", &props, true);
        assert!(output.contains("interface ButtonProps"));
        assert!(output.contains("size?: 'sm' | 'md'"));
        assert!(output.contains("variant?: 'primary' | 'secondary'"));
        assert!(output.contains("children?: React.ReactNode"));
    }

    #[test]
    fn test_generate_prop_interface_no_children() {
        let props = vec![VariantProp {
            name: "size".into(),
            values: vec!["sm".into()],
            default: "sm".into(),
        }];
        let output = generate_prop_interface("Icon", &props, false);
        assert!(output.contains("interface IconProps"));
        assert!(!output.contains("children"));
    }

    #[test]
    fn test_generate_destructure() {
        let props = vec![
            VariantProp {
                name: "size".into(),
                values: vec!["sm".into(), "md".into()],
                default: "md".into(),
            },
            VariantProp {
                name: "variant".into(),
                values: vec!["primary".into()],
                default: "primary".into(),
            },
        ];
        let output = generate_destructure(&props, true);
        assert!(output.contains("size = 'md'"));
        assert!(output.contains("variant = 'primary'"));
        assert!(output.contains("children"));
    }
}
