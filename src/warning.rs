use std::fmt;

#[derive(Debug, Clone)]
pub struct Warning {
    pub node_id: String,
    pub node_name: String,
    pub message: String,
}

impl fmt::Display for Warning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[WARN] node \"{}\" (id: {}): {}",
            self.node_name, self.node_id, self.message
        )
    }
}

#[derive(Debug, Default)]
pub struct WarningCollector {
    warnings: Vec<Warning>,
}

impl WarningCollector {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn warn(&mut self, node_id: &str, node_name: &str, message: &str) {
        self.warnings.push(Warning {
            node_id: node_id.to_string(),
            node_name: node_name.to_string(),
            message: message.to_string(),
        });
    }

    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    pub fn warnings(&self) -> &[Warning] {
        &self.warnings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collector_starts_empty() {
        let collector = WarningCollector::new();
        assert!(!collector.has_warnings());
        assert_eq!(collector.warnings().len(), 0);
    }

    #[test]
    fn test_add_warning() {
        let mut collector = WarningCollector::new();
        collector.warn(
            "node-1",
            "Button",
            "angular gradient converted to linear approximation",
        );
        assert!(collector.has_warnings());
        assert_eq!(collector.warnings().len(), 1);
    }

    #[test]
    fn test_warning_format() {
        let mut collector = WarningCollector::new();
        collector.warn("abc123", "IconBadge", "unsupported blend mode: multiply");
        let formatted = collector.warnings()[0].to_string();
        assert_eq!(
            formatted,
            r#"[WARN] node "IconBadge" (id: abc123): unsupported blend mode: multiply"#
        );
    }

    #[test]
    fn test_multiple_warnings() {
        let mut collector = WarningCollector::new();
        collector.warn("a", "A", "warning 1");
        collector.warn("b", "B", "warning 2");
        collector.warn("c", "C", "warning 3");
        assert_eq!(collector.warnings().len(), 3);
    }
}
