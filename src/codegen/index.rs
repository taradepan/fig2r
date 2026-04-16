/// Generate index.ts re-exports. Each pair is (export_name, file_stem).
pub fn generate_index(components: &[(&str, &str)]) -> String {
    components
        .iter()
        .map(|(name, stem)| format!("export {{ {name} }} from './{stem}';\n"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_export() {
        let output = generate_index(&[("Button", "Button")]);
        assert_eq!(output, "export { Button } from './Button';\n");
    }

    #[test]
    fn test_kebab_export() {
        let output = generate_index(&[("Button", "button")]);
        assert_eq!(output, "export { Button } from './button';\n");
    }

    #[test]
    fn test_multiple_exports() {
        let output =
            generate_index(&[("Avatar", "Avatar"), ("Button", "Button"), ("Card", "Card")]);
        assert!(output.contains("export { Avatar } from './Avatar';"));
        assert!(output.contains("export { Button } from './Button';"));
        assert!(output.contains("export { Card } from './Card';"));
    }

    #[test]
    fn test_empty() {
        let output = generate_index(&[]);
        assert_eq!(output, "");
    }
}
