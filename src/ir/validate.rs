use crate::error::Fig2rError;
use crate::ir::schema::{DesignIR, Node};

pub fn validate_ir(ir: &DesignIR) -> Result<(), Fig2rError> {
    if ir.version.is_empty() {
        return Err(Fig2rError::Message("IR version is required".into()));
    }

    for (i, node) in ir.components.iter().enumerate() {
        validate_node(node, &format!("components[{i}]"))?;
    }

    Ok(())
}

fn validate_node(node: &Node, path: &str) -> Result<(), Fig2rError> {
    if node.id.is_empty() {
        return Err(Fig2rError::Message(format!("{path}: node id is required")));
    }
    if node.name.is_empty() {
        return Err(Fig2rError::Message(format!(
            "{path}: node name is required"
        )));
    }
    for (i, child) in node.children.iter().enumerate() {
        validate_node(child, &format!("{path}.children[{i}]"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::schema::NodeType;

    #[test]
    fn test_valid_ir() {
        let ir = DesignIR {
            version: "1.0".into(),
            name: "Test".into(),
            theme: None,
            assets: vec![],
            components: vec![Node {
                id: "a".into(),
                name: "Box".into(),
                node_type: NodeType::Frame,
                layout: None,
                style: None,
                text: None,
                vector: None,
                vector_paths: None,
                boolean_op: None,
                mask: None,
                component: None,
                children: vec![],
                overlay: false,
            }],
        };
        assert!(validate_ir(&ir).is_ok());
    }

    #[test]
    fn test_missing_version() {
        let ir = DesignIR {
            version: "".into(),
            name: "Test".into(),
            theme: None,
            assets: vec![],
            components: vec![],
        };
        let result = validate_ir(&ir);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("version"));
    }

    #[test]
    fn test_node_missing_id() {
        let ir = DesignIR {
            version: "1.0".into(),
            name: "Test".into(),
            theme: None,
            assets: vec![],
            components: vec![Node {
                id: "".into(),
                name: "Box".into(),
                node_type: NodeType::Frame,
                layout: None,
                style: None,
                text: None,
                vector: None,
                vector_paths: None,
                boolean_op: None,
                mask: None,
                component: None,
                children: vec![],
                overlay: false,
            }],
        };
        let result = validate_ir(&ir);
        assert!(result.is_err());
    }

    #[test]
    fn test_node_missing_name() {
        let ir = DesignIR {
            version: "1.0".into(),
            name: "Test".into(),
            theme: None,
            assets: vec![],
            components: vec![Node {
                id: "a".into(),
                name: "".into(),
                node_type: NodeType::Frame,
                layout: None,
                style: None,
                text: None,
                vector: None,
                vector_paths: None,
                boolean_op: None,
                mask: None,
                component: None,
                children: vec![],
                overlay: false,
            }],
        };
        let result = validate_ir(&ir);
        assert!(result.is_err());
    }
}
