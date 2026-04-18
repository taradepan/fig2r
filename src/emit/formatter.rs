/// Pre-computed indent strings for levels 0..=32. Deeper levels (very rare in real
/// JSX trees) are handled by slicing into `DEEP`.
const DEEP: &str = "                                                                                                                                "; // 128 spaces
const INDENTS: [&str; 33] = [
    "",
    "  ",
    "    ",
    "      ",
    "        ",
    "          ",
    "            ",
    "              ",
    "                ",
    "                  ",
    "                    ",
    "                      ",
    "                        ",
    "                          ",
    "                            ",
    "                              ",
    "                                ",
    "                                  ",
    "                                    ",
    "                                      ",
    "                                        ",
    "                                          ",
    "                                            ",
    "                                              ",
    "                                                ",
    "                                                  ",
    "                                                    ",
    "                                                      ",
    "                                                        ",
    "                                                          ",
    "                                                            ",
    "                                                              ",
    "                                                                ",
];

pub fn indent(level: usize) -> &'static str {
    if let Some(s) = INDENTS.get(level) {
        return s;
    }
    let spaces = (level * 2).min(DEEP.len());
    &DEEP[..spaces]
}

pub fn to_pascal_case(s: &str) -> String {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    let upper: String = first.to_uppercase().collect();
                    let rest: String = chars.collect::<String>().to_lowercase();
                    format!("{upper}{rest}")
                }
                None => String::new(),
            }
        })
        .collect()
}

pub fn to_kebab_case(s: &str) -> String {
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 {
                result.push('-');
            }
            result.extend(ch.to_lowercase());
        } else {
            result.push(ch);
        }
    }
    result
}

pub fn sanitize_component_name(name: &str) -> String {
    let pascal = to_pascal_case(name);
    if pascal.is_empty() {
        return "Component".to_string();
    }
    if pascal.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        return format!("Component{pascal}");
    }
    pascal
}

pub fn join_classes(classes: &[String]) -> String {
    classes.join(" ")
}

/// Escape text content for safe JSX embedding.
/// Replaces characters that would break JSX: < > { } &
pub fn escape_jsx_text(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('{', "&#123;")
        .replace('}', "&#125;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_indent_zero() {
        assert_eq!(indent(0), "");
    }

    #[test]
    fn test_indent_two() {
        assert_eq!(indent(2), "    ");
    }

    #[test]
    fn test_indent_deep_fallback() {
        assert!(indent(100).chars().all(|c| c == ' '));
    }

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("my-button"), "MyButton");
        assert_eq!(to_pascal_case("icon_check"), "IconCheck");
        assert_eq!(to_pascal_case("already PascalCase"), "AlreadyPascalcase");
        assert_eq!(to_pascal_case("hello world"), "HelloWorld");
        assert_eq!(to_pascal_case("Button"), "Button");
    }

    #[test]
    fn test_to_kebab_case() {
        assert_eq!(to_kebab_case("MyButton"), "my-button");
        assert_eq!(to_kebab_case("IconCheck"), "icon-check");
        assert_eq!(to_kebab_case("already-kebab"), "already-kebab");
    }

    #[test]
    fn test_sanitize_component_name() {
        assert_eq!(sanitize_component_name("My Button!"), "MyButton");
        assert_eq!(sanitize_component_name("123start"), "Component123start");
        assert_eq!(sanitize_component_name("icon/check"), "IconCheck");
    }

    #[test]
    fn test_join_classes() {
        let classes = vec![
            "flex".to_string(),
            "flex-row".to_string(),
            "gap-2".to_string(),
        ];
        assert_eq!(join_classes(&classes), "flex flex-row gap-2");
    }

    #[test]
    fn test_join_classes_empty() {
        let classes: Vec<String> = vec![];
        assert_eq!(join_classes(&classes), "");
    }

    #[test]
    fn test_escape_jsx_text() {
        assert_eq!(escape_jsx_text("hello"), "hello");
        assert_eq!(escape_jsx_text("a < b"), "a &lt; b");
        assert_eq!(escape_jsx_text("a > b"), "a &gt; b");
        assert_eq!(escape_jsx_text("{value}"), "&#123;value&#125;");
        assert_eq!(escape_jsx_text("Tom & Jerry"), "Tom &amp; Jerry");
        assert_eq!(
            escape_jsx_text("Price: $5 < $10 & {discount}"),
            "Price: $5 &lt; $10 &amp; &#123;discount&#125;"
        );
    }
}
