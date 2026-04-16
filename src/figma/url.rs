use crate::error::Fig2rError;

/// Parsed Figma URL components
#[derive(Debug, Clone)]
pub struct FigmaRef {
    pub file_key: String,
    pub node_id: Option<String>,
}

/// Parse a Figma URL into file key and optional node ID.
///
/// Supports formats:
/// - https://www.figma.com/design/FILE_KEY/Name?node-id=1234-5678
/// - https://www.figma.com/file/FILE_KEY/Name?node-id=1234-5678
/// - https://www.figma.com/design/FILE_KEY/Name
/// - Just a file key: FILE_KEY
pub fn parse_figma_url(input: &str) -> Result<FigmaRef, Fig2rError> {
    // If it's just a file key (no slashes, no dots)
    if !input.contains('/') && !input.contains('.') {
        return Ok(FigmaRef {
            file_key: input.to_string(),
            node_id: None,
        });
    }

    // Parse URL
    let url = input.trim();

    // Extract file key from path: /design/FILE_KEY/... or /file/FILE_KEY/...
    let file_key = extract_path_segment(url, "design")
        .or_else(|| extract_path_segment(url, "file"))
        .ok_or_else(|| Fig2rError::Message(format!("Cannot parse Figma URL: {url}")))?;

    // Extract node-id from query params
    let node_id = extract_query_param(url, "node-id").map(|id| id.replace('-', ":"));

    Ok(FigmaRef { file_key, node_id })
}

fn extract_path_segment(url: &str, after: &str) -> Option<String> {
    let pattern = format!("/{after}/");
    let start = url.find(&pattern)?;
    let rest = &url[start + pattern.len()..];
    let end = rest.find('/').unwrap_or(rest.len());
    let key = &rest[..end];
    // Strip query params if no trailing slash
    let key = key.split('?').next().unwrap_or(key);
    if key.is_empty() {
        None
    } else {
        Some(key.to_string())
    }
}

fn extract_query_param(url: &str, param: &str) -> Option<String> {
    let query_start = url.find('?')?;
    let query = &url[query_start + 1..];
    for part in query.split('&') {
        if let Some((key, value)) = part.split_once('=')
            && key == param
        {
            return Some(value.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_design_url_with_node() {
        let url = "https://www.figma.com/design/Dd813T8UynWQpm00mv2Tt6/Get-Started-Page?node-id=4387-2085&m=dev";
        let r = parse_figma_url(url).unwrap();
        assert_eq!(r.file_key, "Dd813T8UynWQpm00mv2Tt6");
        assert_eq!(r.node_id.unwrap(), "4387:2085");
    }

    #[test]
    fn test_parse_file_url() {
        let url = "https://www.figma.com/file/ABC123/MyFile?node-id=1-2";
        let r = parse_figma_url(url).unwrap();
        assert_eq!(r.file_key, "ABC123");
        assert_eq!(r.node_id.unwrap(), "1:2");
    }

    #[test]
    fn test_parse_url_no_node() {
        let url = "https://www.figma.com/design/ABC123/MyFile";
        let r = parse_figma_url(url).unwrap();
        assert_eq!(r.file_key, "ABC123");
        assert!(r.node_id.is_none());
    }

    #[test]
    fn test_parse_bare_key() {
        let r = parse_figma_url("Dd813T8UynWQpm00mv2Tt6").unwrap();
        assert_eq!(r.file_key, "Dd813T8UynWQpm00mv2Tt6");
        assert!(r.node_id.is_none());
    }

    #[test]
    fn test_parse_invalid() {
        let r = parse_figma_url("https://example.com/something");
        assert!(r.is_err());
    }
}
